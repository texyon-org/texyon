const MAX_HLIST_STACK: i16 = 512;

// translated from xetex-linebreak.c:2827 (push_node)
#[no_mangle]
pub unsafe extern "C" fn push_node(p: i32) {
    use crate::globals::{hlist_stack_level, set_hlist_stack_entry, set_hlist_stack_level};

    let level = hlist_stack_level();
    if level > MAX_HLIST_STACK {
        crate::errors::pdf_error(c"push_node".as_ptr(), c"stack overflow".as_ptr());
    }

    set_hlist_stack_entry(level, p);
    set_hlist_stack_level(level + 1);
}

// translated from xetex-linebreak.c:2837 (pop_node)
#[no_mangle]
pub unsafe extern "C" fn pop_node() -> i32 {
    use crate::globals::{hlist_stack_entry, hlist_stack_level, set_hlist_stack_level};

    let level = hlist_stack_level() - 1;
    set_hlist_stack_level(level);
    if level < 0 {
        crate::errors::pdf_error(c"pop_node".as_ptr(), c"stack underflow (internal error)".as_ptr());
    }

    hlist_stack_entry(level)
}
