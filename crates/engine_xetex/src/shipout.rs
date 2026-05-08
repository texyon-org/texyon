//! Shipout helpers translated from `xetex/xetex-shipout.c`.

use crate::globals::{set_mem_b16s0, set_mem_b16s1, set_mem_b32s1};
use crate::mem::get_node;

// xetex-constants.h:105,114. EDGE_NODE is an alias of STYLE_NODE.
const EDGE_NODE: u16 = 14;
const EDGE_NODE_SIZE: i32 = 3;

/// Create a new edge node of subtype `s` and width `w`.
///
/// translated from xetex-shipout.c:1708 (new_edge)
#[no_mangle]
pub unsafe extern "C" fn new_edge(s: i16, w: i32) -> i32 {
    let p = get_node(EDGE_NODE_SIZE);
    set_mem_b16s1(p, EDGE_NODE);
    set_mem_b16s0(p, s as u16);
    set_mem_b32s1(p + 1, w);
    set_mem_b32s1(p + 2, 0);
    p
}
