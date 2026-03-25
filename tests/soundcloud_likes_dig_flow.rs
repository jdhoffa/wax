use wax::cli::SortMode;
use wax::model::{ItemKind, Platform, SeedAlbum};
use wax::score::{rank_candidates, ScoreOptions};
use wax::soundcloud::{parse_likers, parse_user_likes_page};

#[test]
fn fixture_soundcloud_likes_flow_produces_overlap_result() {
    let seed = SeedAlbum {
        platform: Platform::Soundcloud,
        kind: ItemKind::Track,
        title: "Seed Track".to_string(),
        artist: "Seed User".to_string(),
        url: "https://soundcloud.com/seed-user/seed-track".to_string(),
        artist_url: Some("https://soundcloud.com/seed-user".to_string()),
        tags: vec!["ambient".to_string()],
        label: None,
        release_id: Some("100".to_string()),
    };

    let likers = parse_likers(include_str!("fixtures/soundcloud_likers.json")).unwrap();
    let source_a = parse_user_likes_page(
        include_str!("fixtures/soundcloud_user_likes_a.json"),
        &likers[0],
        "100",
        2,
    )
    .unwrap()
    .source
    .unwrap();
    let source_b = parse_user_likes_page(
        include_str!("fixtures/soundcloud_user_likes_b.json"),
        &likers[1],
        "100",
        2,
    )
    .unwrap()
    .source
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
            source_label_plural: "likers",
            sort: SortMode::Score,
            limit: 10,
        },
    );

    assert_eq!(ranked[0].title, "Related One");
    assert_eq!(ranked[0].artist, "Artist A");
    assert_eq!(ranked[0].overlap_count, 2);
    assert!(ranked[0].reason.contains("sampled likers"));
}
