#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::restriction)]

use std::env;

mod paths {
    use std::env;
    use std::path::PathBuf;

    pub fn crate_root() -> PathBuf {
        let root = env::var_os("CARGO_MANIFEST_DIR").expect("cargo-provided CARGO_MANIFEST_DIR env variable not set");
        PathBuf::from(root)
    }

    pub fn mruby_root() -> PathBuf {
        crate_root().join("vendor").join("mruby")
    }

    pub fn mrbgems_root() -> PathBuf {
        crate_root().join("cext").join("mrbgems")
    }

    pub fn mrbsys_root() -> PathBuf {
        crate_root().join("cext").join("mrbsys")
    }

    pub fn emscripten_root() -> PathBuf {
        crate_root().join("vendor").join("emscripten")
    }

    pub fn bindgen_header() -> PathBuf {
        crate_root().join("cext").join("bindgen.h")
    }
}

mod libs {
    use std::env;
    use std::ffi::OsStr;
    use std::path::PathBuf;
    use std::process::{Command, ExitStatus, Stdio};
    use std::str;

    use super::paths;
    use crate::Wasm;

    fn mruby_sources() -> impl Iterator<Item = PathBuf> {
        [
            "src/array.c",
            "src/backtrace.c",
            "src/class.c",
            "src/codedump.c",
            "src/compar.c",
            "src/debug.c",
            "src/dump.c",
            "src/enum.c",
            "src/error.c",
            "src/etc.c",
            "src/fmt_fp.c",
            "src/gc.c",
            "src/hash.c",
            "src/init.c",
            "src/kernel.c",
            "src/load.c",
            "src/numeric.c",
            "src/object.c",
            "src/pool.c",
            "src/print.c",
            "src/proc.c",
            "src/range.c",
            "src/readflt.c",
            "src/readint.c",
            "src/state.c",
            "src/string.c",
            "src/symbol.c",
            "src/variable.c",
            "src/version.c",
            "src/vm.c",
        ]
        .into_iter()
        .map(|source| paths::mruby_root().join(source))
    }

    fn mruby_include_dirs() -> impl Iterator<Item = PathBuf> {
        [
            "include", // mruby core
        ]
        .into_iter()
        .map(|dir| paths::mruby_root().join(dir))
    }

    fn mrbgems_sources() -> impl Iterator<Item = PathBuf> {
        [
            "mrbgems/mruby-class-ext/src/class.c",   // NOTE(GH-32): Pending removal.
            "mrbgems/mruby-compiler/core/codegen.c", // Ruby parser and bytecode generation
            "mrbgems/mruby-compiler/core/y.tab.c",   // Ruby parser and bytecode generation
            "mrbgems/mruby-error/src/exception.c",   // `mrb_raise`, `mrb_protect`
            "mrbgems/mruby-eval/src/eval.c",         // eval, instance_eval, and friends
            "mrbgems/mruby-fiber/src/fiber.c",       // Fiber class from core, required by `Enumerator`
            "mrbgems/mruby-metaprog/src/metaprog.c", // APIs on Kernel and Module for accessing classes and variables
            "mrbgems/mruby-method/src/method.c",     // `Method`, `UnboundMethod`, and method APIs on Kernel and Module
            "mrbgems/mruby-pack/src/pack.c",         // Array#pack and String#unpack
            "mrbgems/mruby-proc-ext/src/proc.c",     // NOTE(GH-32): This gem is required by `mruby-method`.
            "mrbgems/mruby-sprintf/src/sprintf.c",   // Kernel#sprintf, Kernel#format, String#%
        ]
        .into_iter()
        .map(|source| paths::mruby_root().join(source))
        .chain(
            ["src/gem_init.c", "src/mrbgems.c"]
                .into_iter()
                .map(|source| paths::mrbgems_root().join(source)),
        )
    }

    fn mrbgems_include_dirs() -> impl Iterator<Item = PathBuf> {
        [
            "mrbgems/mruby-class-ext/include", // NOTE(GH-32): Pending removal.
            "mrbgems/mruby-compiler/core",     // Ruby parser and bytecode generation
            "mrbgems/mruby-error/include",     // `mrb_raise`, `mrb_protect`
            "mrbgems/mruby-eval/include",      // eval, instance_eval, and friends
            "mrbgems/mruby-fiber/include",     // Fiber class from core, required by `Enumerator`
            "mrbgems/mruby-metaprog/include",  // APIs on Kernel and Module for accessing classes and variables
            "mrbgems/mruby-method/include",    // `Method`, `UnboundMethod`, and method APIs on Kernel and Module
            "mrbgems/mruby-pack/include",      // Array#pack and String#unpack
            "mrbgems/mruby-proc-ext/include",  // NOTE(GH-32): This gem is required by `mruby-method`.
            "mrbgems/mruby-sprintf/include",   // Kernel#sprintf, Kernel#format, String#%
        ]
        .into_iter()
        .map(|dir| paths::mruby_root().join(dir))
        .chain(mruby_include_dirs())
    }

    fn mrbsys_sources() -> impl Iterator<Item = PathBuf> {
        ["src/ext.c"]
            .into_iter()
            .map(|source| paths::mrbsys_root().join(source))
    }

    fn mrbsys_include_dirs() -> impl Iterator<Item = PathBuf> {
        ["include"]
            .into_iter()
            .map(|dir| paths::mrbsys_root().join(dir))
            .chain(mruby_include_dirs())
    }

    // From `emsdk/upstream/emscripten/tools/system_libs.py` in emsdk 3.1.12:
    fn wasm_include_dirs() -> impl Iterator<Item = PathBuf> {
        [
            "system/include/compat",
            "system/include",
            "system/lib/libc/musl/include",
            "system/lib/libc/musl/arch/emscripten",
            "system/lib/compiler-rt/include",
            "system/lib/libunwind/include",
        ]
        .into_iter()
        .map(|dir| paths::emscripten_root().join(dir))
    }

    fn staticlib(
        wasm: Option<Wasm>,
        name: &str,
        include_dirs: impl Iterator<Item = PathBuf>,
        sources: impl Iterator<Item = PathBuf>,
    ) {
        assert!(
            name.starts_with("lib"),
            "Static lib name must be of the format libXXX.a, got {name}"
        );
        assert_eq!(
            name.rsplit('.').next().map(|ext| ext.eq_ignore_ascii_case("a")),
            Some(true),
            "Static lib name must be of the format libXXX.a, got {name}"
        );
        assert!(
            name.len() > 5,
            "Static lib name must be of the format libXXX.a, got {name}"
        );

        let mut build = cc::Build::new();
        build
            .warnings(false)
            .define("ARTICHOKE", None)
            .define("MRB_ARY_NO_EMBED", None)
            .define("MRB_GC_TURN_OFF_GENERATIONAL", None)
            .define("MRB_INT64", None)
            .define("MRB_NO_BOXING", None)
            .define("MRB_NO_PRESYM", None)
            .define("MRB_NO_STDIO", None)
            .define("MRB_UTF8_STRING", None);

        for source in sources {
            println!("cargo:rerun-if-changed={}", source.to_str().unwrap());
            // relative paths ensure that `cc` will preserve directory hierarchy
            // which allows C sources with the same file name to be built.
            let source = source
                .strip_prefix(paths::crate_root())
                .expect("All C sources are found within the crate root");
            build.file(source);
        }

        for include_dir in include_dirs {
            build.include(include_dir);
        }

        match wasm {
            Some(Wasm::Emscripten) => {
                for include_dir in wasm_include_dirs() {
                    build.include(include_dir);
                }
                build.define("MRB_API", Some(r#"__attribute__((used))"#));
            }
            Some(Wasm::Unknown) => {
                for include_dir in wasm_include_dirs() {
                    build.include(include_dir);
                }
                build.define("MRB_API", Some(r#"__attribute__((visibility("default")))"#));
                build.define("MRB_NO_DIRECT_THREADING", None);
            }
            None => {}
        }

        build.compile(name);
    }

    fn bindgen(wasm: Option<Wasm>, out_dir: &OsStr) {
        // Try to use an existing global install of bindgen
        let status = invoke_bindgen(wasm, out_dir, OsStr::new("bindgen"));
        if matches!(status, Some(status) if status.success()) {
            return;
        }
        // Install bindgen
        // cargo install --root target/bindgen --version 0.59.1 bindgen
        let bindgen_install_dir = PathBuf::from(out_dir).join("bindgen");
        let status = Command::new(env::var_os("CARGO").unwrap())
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .arg("install")
            .arg("--root")
            .arg(&bindgen_install_dir)
            .arg("--version")
            .arg("0.59.1")
            .arg("bindgen")
            .status()
            .unwrap();
        assert!(status.success(), "cargo install bindgen failed");

        let status = invoke_bindgen(
            wasm,
            out_dir,
            bindgen_install_dir.join("bin").join("bindgen").as_os_str(),
        );
        assert!(status.unwrap().success(), "bindgen failed");
    }

    pub fn invoke_bindgen(wasm: Option<Wasm>, out_dir: &OsStr, bindgen_executable: &OsStr) -> Option<ExitStatus> {
        let bindings_out_path = PathBuf::from(out_dir).join("ffi.rs");
        let mut command = Command::new(bindgen_executable);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        command
            .arg("--allowlist-function")
            .arg("^mrb.*")
            .arg("--allowlist-type")
            .arg("^mrb.*")
            .arg("--allowlist-var")
            .arg("^mrb.*")
            .arg("--allowlist-var")
            .arg("^MRB.*")
            .arg("--allowlist-var")
            .arg("^MRUBY.*")
            .arg("--allowlist-var")
            .arg("REGEXP_CLASS")
            .arg("--rustified-enum")
            .arg("mrb_vtype")
            .arg("--rustified-enum")
            .arg("mrb_lex_state_enum")
            .arg("--rustified-enum")
            .arg("mrb_range_beg_len")
            .arg("--no-doc-comments")
            .arg("--size_t-is-usize")
            .arg("--output")
            .arg(bindings_out_path)
            .arg(paths::bindgen_header())
            .arg("--")
            .arg("-DARTICHOKE")
            .arg("-DMRB_ARY_NO_EMBED")
            .arg("-DMRB_GC_TURN_OFF_GENERATIONAL")
            .arg("-DMRB_INT64")
            .arg("-DMRB_NO_BOXING")
            .arg("-DMRB_NO_PRESYM")
            .arg("-DMRB_NO_STDIO")
            .arg("-DMRB_UTF8_STRING");

        for include_dir in mruby_include_dirs().chain(mrbsys_include_dirs()) {
            command.arg("-I").arg(include_dir);
        }

        if wasm.is_some() {
            for include_dir in wasm_include_dirs() {
                command.arg("-I").arg(include_dir);
            }
            command.arg(r#"-DMRB_API=__attribute__((visibility("default")))"#);
        }

        command.status().ok()
    }

    pub fn build(wasm: Option<Wasm>, out_dir: &OsStr) {
        staticlib(
            wasm,
            "libartichokemruby.a",
            mruby_include_dirs()
                .chain(mrbgems_include_dirs())
                .chain(mrbsys_include_dirs()),
            mruby_sources().chain(mrbgems_sources()).chain(mrbsys_sources()),
        );
        bindgen(wasm, out_dir);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Wasm {
    Emscripten,
    Unknown,
}

impl Wasm {
    #[must_use]
    pub fn from_env() -> Option<Self> {
        // Ref: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
        let arch =
            env::var_os("CARGO_CFG_TARGET_ARCH").expect("cargo-provided CARGO_CFG_TARGET_ARCH env variable not set");
        if !matches!(arch.to_str(), Some(arch) if arch == "wasm32") {
            return None;
        }
        let os = env::var_os("CARGO_CFG_TARGET_OS").expect("cargo-provided CARGO_CFG_TARGET_OS env variable not set");
        match os.to_str() {
            Some("emscripten") => Some(Self::Emscripten),
            Some("unknown") => Some(Self::Unknown),
            Some(_) | None => None,
        }
    }
}

fn main() {
    let wasm = Wasm::from_env();
    let out_dir = env::var_os("OUT_DIR").expect("cargo-provided OUT_DIR env variable not set");
    libs::build(wasm, &out_dir);
}
