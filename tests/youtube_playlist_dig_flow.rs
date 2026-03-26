use wax::cli::SortMode;
use wax::score::{rank_candidates, ScoreOptions};
use wax::youtube::{build_playlist_source, parse_playlist_page, parse_seed};

#[test]
fn fixture_youtube_playlist_flow_produces_overlap_result() {
    let seed = parse_seed(include_str!("fixtures/youtube_video_seed.json")).unwrap();
    let source_a = build_playlist_source(
        parse_playlist_page(include_str!("fixtures/youtube_playlist_items_pl_one.json")).unwrap(),
        "PL_ONE",
        Some("Late Night Finds"),
        "seed123",
    )
    .unwrap()
    .unwrap();
    let source_b = build_playlist_source(
        parse_playlist_page(include_str!("fixtures/youtube_playlist_items_pl_two.json")).unwrap(),
        "PL_TWO",
        Some("Weekend Rotation"),
        "seed123",
    )
    .unwrap()
    .unwrap();

    let ranked = rank_candidates(
        &seed,
        vec![
            (source_a.title, source_a.tracks),
            (source_b.title, source_b.tracks),
        ],
        &ScoreOptions {
            min_overlap: 1,
            exclude_artist: false,
            exclude_label: false,
            required_tags: vec![],
            source_label_plural: "playlists",
            sort: SortMode::Score,
            limit: 10,
        },
    );

    assert_eq!(ranked[0].title, "Related One");
    assert_eq!(ranked[0].artist, "Artist A");
    assert_eq!(ranked[0].overlap_count, 2);
    assert!(ranked[0].reason.contains("closest distance 1"));
}
