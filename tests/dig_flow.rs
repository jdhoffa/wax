use wax::cli::SortMode;
use wax::parser::{parse_collectors, parse_owned_albums, resolve_seed};
use wax::score::{rank_candidates, ScoreOptions};

#[test]
fn fixture_flow_produces_overlap_result() {
    let seed_html = include_str!("fixtures/seed.html");
    let fan_a = include_str!("fixtures/fan_a.html");
    let fan_b = include_str!("fixtures/fan_b.html");

    let seed = resolve_seed("https://seed.bandcamp.com/album/seed-record", seed_html).unwrap();
    let collectors = parse_collectors(seed_html);
    assert_eq!(collectors.len(), 2);

    let ranked = rank_candidates(
        &seed,
        vec![
            (collectors[0].handle.clone(), parse_owned_albums(fan_a)),
            (collectors[1].handle.clone(), parse_owned_albums(fan_b)),
        ],
        &ScoreOptions {
            min_overlap: 1,
            exclude_artist: true,
            exclude_label: false,
            required_tags: vec![],
            source_label_plural: "collectors",
            sort: SortMode::Score,
            limit: 10,
        },
    );

    assert_eq!(ranked[0].title, "Related One");
    assert_eq!(ranked[0].overlap_count, 2);
}
