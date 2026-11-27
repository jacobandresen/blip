use super::*;

#[test]
fn each_seat_has_distinct_join_and_turn_keys() {
    assert_eq!(seat_keys(0), [BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_SPACE]);
    assert_eq!(seat_keys(1), [BLIP_KEY_LEFT, BLIP_KEY_RIGHT, BLIP_KEY_J]);
}
