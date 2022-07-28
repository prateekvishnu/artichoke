.globl stackSave
.globl stackRestore
.globl stackAlloc
.globl emscripten_stack_get_current

#ifdef __wasm64__
#define PTR i64
#define MASK 0xfffffffffffffff0
#else
#define PTR i32
#define MASK 0xfffffff0
#endif

.globaltype __stack_pointer, PTR

stackSave:
  .functype stackSave() -> (PTR)
  global.get __stack_pointer
  end_function

stackRestore:
  .functype stackRestore(PTR) -> ()
  local.get 0
  global.set __stack_pointer
  end_function

stackAlloc:
  .functype stackAlloc(PTR) -> (PTR)
  .local PTR, PTR
  global.get __stack_pointer
  # Get arg 0 -> number of bytes to allocate
  local.get 0
  # Stack grows down.  Subtract arg0 from __stack_pointer
  PTR.sub
  # Align result by anding with ~15
  PTR.const MASK
  PTR.and
  local.tee 1
  global.set __stack_pointer
  local.get 1
  end_function

emscripten_stack_get_current:
  .functype emscripten_stack_get_current () -> (PTR)
  global.get __stack_pointer
  end_function

