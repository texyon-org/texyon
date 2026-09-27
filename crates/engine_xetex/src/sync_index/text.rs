//! Reading the record stream back from an uncompressed `.synctex` file.

use std::sync::Arc;

use super::{Record, RecordKind, SyncIndex};

pub(super) fn parse(text: &str) -> SyncIndex {
    let mut index = SyncIndex::default();
    let mut unit = 1i32;
    let mut page_index = None;
    let mut open = Vec::new();
    let flush = |index: &mut SyncIndex, open: &mut Vec<Record>| {
        if let Some(record) = open.first() {
            let page_index = record.page_index;
            index.push_page(page_index, Arc::from(std::mem::take(open)));
        }
    };
    for line in text.lines() {
        if let Some(input) = line.strip_prefix("Input:") {
            if let Some((tag, name)) = input.split_once(':') {
                if let Ok(tag) = tag.parse() {
                    index.names.insert(tag, Arc::from(name));
                }
            }
            continue;
        }
        if let Some(value) = line.strip_prefix("Unit:") {
            unit = value.trim().parse().unwrap_or(1);
            continue;
        }
        let mut chars = line.chars();
        let Some(prefix) = chars.next() else {
            continue;
        };
        let rest = chars.as_str();
        match prefix {
            '{' => {
                flush(&mut index, &mut open);
                page_index = rest
                    .parse::<usize>()
                    .ok()
                    .and_then(|page| page.checked_sub(1))
            }
            '}' => {
                flush(&mut index, &mut open);
                page_index = None;
            }
            _ => {
                let (Some(page_index), Some(kind)) = (page_index, RecordKind::from_prefix(prefix))
                else {
                    continue;
                };
                if let Some(record) = parse_record(page_index, kind, rest, unit) {
                    open.push(record);
                }
            }
        }
    }
    flush(&mut index, &mut open);
    index
}

/// `tag,line:h,v:width,height,depth`, with trailing fields optional.
fn parse_record(page_index: usize, kind: RecordKind, rest: &str, unit: i32) -> Option<Record> {
    let mut record = Record::empty(page_index, kind);
    if kind.is_end() {
        return Some(record);
    }
    let mut groups = rest.split(':');
    let pair = |text: Option<&str>| -> Option<(i32, i32)> {
        let (a, b) = text?.split_once(',')?;
        Some((a.parse().ok()?, b.parse().ok()?))
    };
    (record.tag, record.line) = pair(groups.next())?;
    let (h, v) = pair(groups.next())?;
    record.h = h.wrapping_mul(unit);
    record.v = v.wrapping_mul(unit);
    if let Some(dims) = groups.next() {
        let mut dims = dims
            .split(',')
            .map(|d| d.parse::<i32>().unwrap_or(0) * unit);
        record.width = dims.next().unwrap_or(0);
        record.height = dims.next().unwrap_or(0);
        record.depth = dims.next().unwrap_or(0);
    }
    Some(record)
}
