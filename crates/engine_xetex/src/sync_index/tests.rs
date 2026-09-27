use super::*;

const PARAGRAPH: &str = "\
Input:1:chapters/one.tex
{1
[1,1:0,0:1000,1000,0
(1,5:100,200:800,20,5
g1,3:300,200
g1,4:600,200
k1,5:900,200:100
g1,5:900,200
)
]
}1
";

#[test]
fn backward_takes_the_line_of_the_closest_glue() {
    let index = SyncIndex::from_synctex(PARAGRAPH);
    assert_eq!(index.backward(0, 250, 195).unwrap().line, 3);
    assert_eq!(index.backward(0, 650, 195).unwrap().line, 4);
}

#[test]
fn backward_moves_trailing_kern_and_glue_to_the_last_word() {
    let index = SyncIndex::from_synctex(PARAGRAPH);
    assert_eq!(index.backward(0, 880, 195).unwrap().line, 4);
}

#[test]
fn forward_normalizes_implicit_tex_name() {
    let index = SyncIndex::from_synctex(PARAGRAPH);
    let boxes = index.forward("chapters/one", 3);
    assert_eq!(boxes.len(), 1);
    assert_eq!((boxes[0].h, boxes[0].v, boxes[0].width), (100, 200, 800));
}

/// Documents compiled with `tectonic --synctex`, with the answers of TeX Live's
/// `synctex edit` and `synctex view` for sampled points and every source line.
const FIXTURES: &[(&str, &str, &str)] = &[
    (
        "hardwrap",
        include_str!("fixtures/hardwrap.synctex"),
        include_str!("fixtures/hardwrap.expected"),
    ),
    (
        "lists",
        include_str!("fixtures/lists.synctex"),
        include_str!("fixtures/lists.expected"),
    ),
    (
        "math",
        include_str!("fixtures/math.synctex"),
        include_str!("fixtures/math.expected"),
    ),
    (
        "floats",
        include_str!("fixtures/floats.synctex"),
        include_str!("fixtures/floats.expected"),
    ),
    (
        "inputs",
        include_str!("fixtures/inputs.synctex"),
        include_str!("fixtures/inputs.expected"),
    ),
];

fn bp(sp: i32) -> f64 {
    f64::from(sp) / 65_781.76
}

#[test]
fn queries_match_the_synctex_command_line_tool() {
    for (name, synctex, expected) in FIXTURES {
        let index = SyncIndex::from_synctex(synctex);
        let mut checked = 0;
        for query in expected.lines() {
            let fields = query.split(' ').collect::<Vec<_>>();
            match fields[..] {
                ["e", page, h, v, ref want @ ..] => {
                    let page = page.parse::<usize>().unwrap() - 1;
                    let got = index
                        .backward(page, h.parse().unwrap(), v.parse().unwrap())
                        .map(|rect| format!("{} {}", rect.path, rect.line));
                    let want = (want != ["-"]).then(|| want.join(" "));
                    assert_eq!(got, want, "{name}: {query}");
                }
                ["v", path, line, want] => {
                    let got = index.forward(path, line.parse().unwrap());
                    let want = want
                        .split(';')
                        .filter(|found| *found != "-")
                        .map(|found| {
                            found
                                .split(',')
                                .map(|value| value.parse::<f64>().unwrap())
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(got.len(), want.len(), "{name}: {query}");
                    for (rect, want) in got.iter().zip(&want) {
                        let got = [
                            (rect.page_index + 1) as f64,
                            bp(rect.h),
                            bp(rect.v + rect.depth),
                            bp(rect.width),
                            bp(rect.height + rect.depth),
                        ];
                        for (got, want) in got.iter().zip(want) {
                            assert!((got - want).abs() < 0.011, "{name}: {query}: {got:?}");
                        }
                    }
                }
                _ => panic!("{name}: bad fixture line {query}"),
            }
            checked += 1;
        }
        assert!(checked > 100, "{name}");
    }
}
