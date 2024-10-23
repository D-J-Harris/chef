// saves one CPU cycle, see https://darkcoding.net/software/does-it-matter-what-type-i-use/
pub const U8_MAX_USIZE: usize = u8::MAX as usize;
pub const U8_COUNT_USIZE: usize = U8_MAX_USIZE + 1;

pub const CALL_FRAMES_MAX_COUNT: usize = 64;
pub const LOCALS_MAX_COUNT: usize = U8_COUNT_USIZE;
pub const UPVALUES_MAX_COUNT: usize = U8_COUNT_USIZE;
pub const CONSTANTS_MAX_COUNT: usize = U8_COUNT_USIZE;
pub const STACK_VALUES_MAX_COUNT: usize = CALL_FRAMES_MAX_COUNT * U8_COUNT_USIZE;
pub const FUNCTION_ARITY_MAX_COUNT: u8 = u8::MAX;

pub const INIT_STRING: &str = "init";
pub const SUPER_STRING: &str = "super";

pub fn print_function(name: &str) -> String {
    match name.is_empty() {
        true => "<script>".into(),
        false => format!("<fn {}>", name),
    }
}
