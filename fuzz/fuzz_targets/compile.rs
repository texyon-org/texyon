#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate texyon;

fuzz_target!(|data: &[u8]| {
    if let Ok(data) = std::str::from_utf8(data) {
        let _ = texyon::latex_to_pdf(data);
    }
});
