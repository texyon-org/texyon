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
    (
        "beamer",
        include_str!("fixtures/beamer.synctex"),
        include_str!("fixtures/beamer.expected"),
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
                            let mut found = found
                                .split(',')
                                .map(|value| value.parse::<f64>().unwrap())
                                .collect::<Vec<_>>();
                            // The index reports a negative width from the left edge.
                            if found[3] < 0.0 {
                                found[1] += found[3];
                                found[3] = -found[3];
                            }
                            found
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

/// A 16:9 beamer page in big points.
const SLIDE: [f64; 2] = [453.54, 255.12];

fn on_slide(rect: &SyncRect) -> bool {
    let [width, height] = SLIDE;
    bp(rect.h) >= -0.01
        && bp(rect.v - rect.height) >= -0.01
        && bp(rect.h + rect.width) <= width + 0.01
        && bp(rect.v + rect.depth) <= height + 0.01
}

#[test]
fn highlight_inside_a_beamer_frame_scrolls_to_its_first_overlay() {
    let index = SyncIndex::from_synctex(include_str!("fixtures/beamer.synctex"));
    // `synctex view` answers the nearer `\end{frame}`, the title page's.
    assert_eq!(index.forward("main.tex", 14)[0].page_index, 0);
    // Lines 11 to 22 are the body of the frame on pages 2 to 4.
    for line in 11..=22 {
        let found = index.highlight("main.tex", line).unwrap();
        assert_eq!(found.best.page_index, 1, "{line}");
        assert!(on_slide(&found.best), "{line}: {:?}", found.best);
        assert_eq!(found.boxes, [], "{line}");
    }
}

#[test]
fn highlight_keeps_the_innermost_boxes_of_one_page() {
    let index = SyncIndex::from_synctex(include_str!("fixtures/beamer.synctex"));
    let all = index.forward("main.tex", 23);
    assert!(all.iter().any(|rect| rect.page_index != 1));
    let boxes = index.highlight("main.tex", 23).unwrap().boxes;
    assert!(!boxes.is_empty() && boxes.len() * 4 < all.len());
    for rect in &boxes {
        assert_eq!(rect.page_index, 1);
        assert!(rect.has_area() && on_slide(rect), "{rect:?}");
        let mut others = boxes.iter().filter(|other| *other != rect);
        assert!(!others.any(|other| rect.encloses(other)), "{rect:?}");
    }
}

#[test]
fn highlight_of_a_text_line_is_its_synctex_view_boxes() {
    let index = SyncIndex::from_synctex(include_str!("fixtures/hardwrap.synctex"));
    let mut fallbacks = 0;
    for line in 1..=40 {
        let all = index.forward("main.tex", line);
        let found = index.highlight("main.tex", line).unwrap();
        assert_eq!(found.best, all[0], "{line}");
        let mut want: Vec<SyncRect> = Vec::new();
        for rect in all {
            if rect.has_area() && !want.contains(&rect) {
                want.push(rect);
            }
        }
        if want[0].line == line {
            assert_eq!(found.boxes, want, "{line}");
        } else {
            // A line without boxes of its own flashes its neighbour.
            assert!(!found.boxes.is_empty(), "{line}");
            fallbacks += 1;
        }
    }
    assert!(fallbacks > 0);
}

fn two_pages(names: &[(i32, &str)], page_two: &[(i32, i32)]) -> SyncIndex {
    let mut text = String::new();
    for (tag, name) in names {
        text.push_str(&format!("Input:{tag}:{name}\n"));
    }
    text.push_str(
        "{1\n[1,1:0,0:1000,1000,0\n(1,2:100,200:800,20,5\n)\n]\n}1\n{2\n[1,1:0,0:1000,1000,0\n",
    );
    for (tag, line) in page_two {
        text.push_str(&format!("({tag},{line}:100,200:800,20,5\n)\n"));
    }
    text.push_str("]\n}2\n");
    SyncIndex::from_synctex(&text)
}

#[test]
fn a_preserved_tail_with_the_same_files_shares_its_pages() {
    let previous = two_pages(&[(1, "main.tex"), (2, "chap.tex")], &[(2, 7)]);
    let foreground = two_pages(&[(1, "main.tex"), (2, "chap.tex")], &[]);
    let merged = SyncIndex {
        names: foreground.names.clone(),
        pages: foreground.pages[..1].to_vec(),
        tag_pages: foreground.tag_pages.clone(),
    }
    .with_preserved_tail(&previous, 1);
    assert!(Arc::ptr_eq(&merged.pages[1], &previous.pages[1]));
    assert_eq!(merged.forward("chap.tex", 7)[0].page_index, 1);
}

#[test]
fn a_preserved_tail_follows_renumbered_files() {
    let previous = two_pages(&[(1, "main.tex"), (2, "chap.tex")], &[(1, 5), (2, 7)]);
    let foreground = two_pages(&[(1, "chap.tex")], &[]);
    let merged = SyncIndex {
        names: foreground.names.clone(),
        pages: foreground.pages[..1].to_vec(),
        tag_pages: foreground.tag_pages.clone(),
    }
    .with_preserved_tail(&previous, 1);
    assert_eq!(merged.forward("chap.tex", 7)[0].page_index, 1);
    assert_eq!(merged.forward("main.tex", 5)[0].page_index, 1);
}

#[test]
fn a_renamed_tag_without_tail_records_still_shares_the_tail() {
    // The second input of main.tex typesets nothing below the stale page.
    let previous = two_pages(&[(1, "main.tex"), (2, "main.tex")], &[(1, 7)]);
    let foreground = two_pages(&[(1, "main.tex")], &[]);
    let merged = SyncIndex {
        names: foreground.names.clone(),
        pages: foreground.pages[..1].to_vec(),
        tag_pages: foreground.tag_pages.clone(),
    }
    .with_preserved_tail(&previous, 1);
    assert!(Arc::ptr_eq(&merged.pages[1], &previous.pages[1]));
    assert_eq!(merged.forward("main.tex", 7)[0].page_index, 1);
}
