// saves one CPU cycle, see https://darkcoding.net/software/does-it-matter-what-type-i-use/
pub const U8_MAX_USIZE: usize = u8::MAX as usize;
pub const U8_COUNT_USIZE: usize = U8_MAX_USIZE + 1;

pub const FRAMES_MAX_COUNT: usize = 64;
pub const LOCALS_MAX_COUNT: usize = U8_COUNT_USIZE;
pub const UPVALUES_MAX_COUNT: usize = U8_COUNT_USIZE;
pub const STACK_VALUES_MAX_COUNT: usize = FRAMES_MAX_COUNT * U8_COUNT_USIZE;

pub const JUMP_MAX_COUNT: u8 = u8::MAX;
pub const JUMP_MAX_COUNT_USIZE: usize = JUMP_MAX_COUNT as usize;
pub const FUNCTION_ARITY_MAX_COUNT: u8 = u8::MAX;

pub const INIT_STRING: &str = "init";
pub const SUPER_STRING: &str = "super";
