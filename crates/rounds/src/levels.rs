//! The round and the week: one street of terraced houses, the same every day, and seven days that each bring
//! their own weather and their own hazards along it.

/// Which side of the street.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side { Left, Right }

impl Side {
    pub const BOTH: [Side; 2] = [Side::Left, Side::Right];
    /// -1 for the left, +1 for the right: the way a throw goes.
    pub fn sign(self) -> f32 { if self == Side::Left { -1.0 } else { 1.0 } }
    pub fn index(self) -> usize { self as usize }
}

/// What stands on one lot of the street.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lot {
    /// A terraced house; `drive` gives it a driveway with a car in it.
    House { drive: bool },
    Bakery,
    Park,
    /// A side street crossing the route.
    Cross,
    /// Dronning Louises Bro over the Lakes: water on both sides and railings that narrow the street.
    Bridge,
    /// Rundetaarn, the round tower.
    Tower,
    /// Marmorkirken, the Marble Church, its green copper dome.
    Church,
    /// Amalienborg: the palace square, Frederik V on his horse in the middle of it.
    Palace,
    /// The mouth of the street the round turns left into: its road, where a house would be.
    Mouth,
    /// The corner building of the turn, seen from the new street: the old street's houses stand there.
    Corner,
}

impl Lot {
    pub fn is_house(self) -> bool { matches!(self, Lot::House { .. }) }
}

/// The route, one character per lot from the start: H a house that takes the paper, h one that does not yet,
/// D and d the same with a driveway, B a bakery, P a park, C a side street, = Dronning Louises Bro, R Rundetaarn,
/// M Marmorkirken, A Amalienborg. Real streets, each landmark on its real side: down Noerrebrogade past Assistens
/// Kirkegaard, over Dronning Louises Bro, along Koebmagergade by Rundetaarn, down Store Kongensgade with Nyboder on
/// the left, then the left turn into Frederiksgade, past Marmorkirken to Amalienborg and the harbour.
pub const ROUTE: [&str; 2] = [
    "PPHhhDhhBhHCHdhHhhhH==hRdHhBhJJXHdhMhdhHAAHhPP",
    "PPhhDhhHhHdChhHBhdPh==HdhHhhHhHdhhHhhhHhAADhPP",
];

/// Where the street is: the name shown as the bike rides into it, from that row on.
pub const PLACES: &[(usize, &str)] = &[
    (0, "NOERREBROGADE"), (11, "BLAAGAARDSGADE"), (12, "NOERREBROGADE"), (18, "ASSISTENS KIRKEGAARD"),
    (20, "DRONNING LOUISES BRO"), (22, "KOEBMAGERGADE"), (23, "RUNDETAARN"), (25, "STORE KONGENSGADE"), (26, "NYBODER"),
    (30, "FREDERIKSGADE"), (35, "MARMORKIRKEN"), (36, "FREDERIKSGADE"), (40, "AMALIENBORG SLOTSPLADS"), (42, "AMALIEHAVEN"),
];

/// The street the round turns into at the left turn.
pub const TURN_INTO: &str = "FREDERIKSGADE";

/// The part of the city a row is in, which sets how its houses look.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum District { Norrebro, IndreBy, Nyboder, Frederiksstaden }

/// Store Kongensgade has Nyboder on its east side only, the left going south; across from it are town houses.
pub fn district(side: Side, row: usize) -> District {
    match row {
        0..=21 => District::Norrebro,
        22..=24 => District::IndreBy,
        25..=29 if side == Side::Left => District::Nyboder,
        25..=29 => District::IndreBy,
        _ => District::Frederiksstaden,
    }
}

/// Bus stops: a shelter on the pavement, people waiting, and a bus at the stop as the bike comes.
pub const STOPS: &[(usize, Side)] = &[(6, Side::Right), (14, Side::Left), (26, Side::Right), (33, Side::Left), (37, Side::Right)];

/// The junctions with traffic lights: the side streets.
pub const JUNCTIONS: [usize; 1] = [11];
/// Where the round turns left out of Store Kongensgade into Frederiksgade: the side street's mouth fills these two
/// rows on the left, and the new street begins after them.
pub const TURN_ROW: usize = 28;
pub const NEW_STREET_ROW: usize = 30;

/// The row of Amalienborg's equestrian statue, which stands in the middle of the road at the start of that row.
pub const STATUE_ROW: usize = 41;

pub const ROWS: usize = ROUTE[0].len();

/// The lot at `row` on `side`, and whether its house starts the week on the round.
pub fn lot(side: Side, row: usize) -> (Lot, bool) {
    match ROUTE[side.index()].as_bytes()[row] {
        b'H' => (Lot::House { drive: false }, true),
        b'h' => (Lot::House { drive: false }, false),
        b'D' => (Lot::House { drive: true }, true),
        b'd' => (Lot::House { drive: true }, false),
        b'B' => (Lot::Bakery, false),
        b'C' => (Lot::Cross, false),
        b'=' => (Lot::Bridge, false),
        b'R' => (Lot::Tower, false),
        b'M' => (Lot::Church, false),
        b'A' => (Lot::Palace, false),
        b'J' => (Lot::Mouth, false),
        b'X' => (Lot::Corner, false),
        _ => (Lot::Park, false),
    }
}

/// What the day does to the ride.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Weather {
    Clear,
    /// Wind along the street, px/s: negative blows a thrown paper back toward where the bike came from.
    Wind(f32),
    /// Papers skid on landing.
    Rain,
    /// Houses show late; their lit doorways show first.
    Fog,
    /// Market stalls and a crowd, and a slower top speed.
    Market,
    /// Heavy Sunday bundles: fewer papers, double points.
    Bundles,
}

/// Something on the street that the bike meets.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Hz {
    /// A car backing out of a driveway (on a D or d lot); its reverse lights glow first.
    Car,
    /// A dog that runs out of a garden gate; the gate rattles first.
    Dog,
    /// A garden sprinkler: it hisses, then sprays over the pavement and bike lane.
    Sprinkler,
    /// A rollerblader coming down the bike lane.
    Blader,
    /// A delivery van: parked in the left bike lane with its hazards on, or driving slowly up the right lane.
    Van,
    /// A puddle at this x: riding through slows the bike.
    Puddle(i16),
    /// Pigeons at this x: they fly off as the bike comes.
    Pigeons(i16),
    /// The newsstand cart, rolling down the bike lane: ride through it for ten more papers.
    Cart,
    /// A market stall over the pavement and bike lane.
    Stall,
    /// Someone crossing the street at a stroll.
    Walker,
    /// A Copenhagen commuter on the cycle track: on the right going the bike's way, slower; on the left coming
    /// toward it (the tracks are one-way, like the traffic beside them).
    Cyclist,
    /// A car on the side street, crossing the junction while the bike's light is red (made by the junction, not
    /// placed by a day).
    CrossCar,
    /// A city bus at a stop: it waits with its doors open while passengers cross the cycle track, then pulls away.
    Bus,
    /// A tourist on a rented e-bike: weaves, stops to look at the screen on the handlebars, then swerves out of the
    /// cycle track and back.
    Tourist,
}

/// One day of the week.
pub struct Day {
    pub name: &'static str,
    pub weather: Weather,
    /// The day's tell, shown before it starts.
    pub tell: &'static str,
    /// What waits along the route: the row, the side and the thing.
    pub hazards: &'static [(u8, Side, Hz)],
    /// Rows where an oncoming car is waiting to drive down the street.
    pub traffic: &'static [u8],
}

use Hz::*;
use Side::{Left as L, Right as R};

pub const DAYS: [Day; 7] = [
    Day {
        name: "MONDAY", weather: Weather::Clear, tell: "A CLEAR MORNING",
        hazards: &[(5, L, Car), (7, R, Pigeons(232)), (10, R, Dog), (14, L, Cart), (17, R, Car), (26, L, Dog),
            (28, R, Pigeons(260)), (33, L, Car), (38, R, Blader), (8, R, Cyclist), (17, L, Cyclist), (30, R, Cyclist),
            (23, R, Tourist)],
        traffic: &[26, 36],
    },
    Day {
        name: "TUESDAY", weather: Weather::Clear, tell: "BINS OUT, SPRINKLERS ON",
        hazards: &[(4, R, Car), (6, L, Sprinkler), (9, R, Dog), (13, L, Car), (15, L, Cart), (16, R, Sprinkler),
            (19, L, Dog), (24, R, Van), (26, L, Blader), (31, R, Car), (34, L, Sprinkler), (37, L, Car), (39, R, Dog),
            (7, L, Cyclist), (12, R, Cyclist), (28, R, Cyclist), (36, L, Cyclist), (22, L, Tourist), (34, R, Tourist)],
        traffic: &[12, 27, 38],
    },
    Day {
        name: "WEDNESDAY", weather: Weather::Wind(-60.0), tell: "WINDY: THROWS DRIFT BACK",
        hazards: &[(5, L, Car), (8, R, Blader), (10, L, Dog), (12, R, Pigeons(250)), (16, L, Cart), (17, R, Car),
            (23, R, Dog), (25, L, Van), (30, R, Blader), (33, L, Car), (36, R, Sprinkler), (42, R, Car), (6, R, Cyclist),
            (14, L, Cyclist), (27, R, Cyclist), (23, R, Tourist), (38, L, Tourist)],
        traffic: &[14, 26, 34],
    },
    Day {
        name: "THURSDAY", weather: Weather::Rain, tell: "RAIN: PAPERS SKID, AIM SHORT",
        hazards: &[(4, R, Car), (6, L, Puddle(200)), (9, R, Puddle(286)), (12, L, Dog), (13, L, Car), (16, R, Cart),
            (18, R, Puddle(230)), (23, R, Car), (25, L, Dog), (28, L, Puddle(190)), (31, R, Car), (33, R, Puddle(262)),
            (36, L, Blader), (37, L, Car), (39, R, Dog), (11, R, Cyclist), (26, L, Cyclist)],
        traffic: &[10, 25, 30, 38],
    },
    Day {
        name: "FRIDAY", weather: Weather::Fog, tell: "FOG: LOOK FOR THE LIT DOORS",
        hazards: &[(5, L, Car), (7, R, Dog), (10, R, Car), (13, L, Blader), (15, L, Cart), (17, R, Car),
            (19, L, Dog), (24, L, Car), (26, R, Blader), (31, R, Car), (34, L, Dog), (38, R, Van), (42, R, Car),
            (12, R, Cyclist), (29, L, Cyclist), (24, L, Tourist), (36, R, Tourist)],
        traffic: &[16, 27, 36],
    },
    Day {
        name: "SATURDAY", weather: Weather::Market, tell: "MARKET DAY: TAKE IT SLOW",
        hazards: &[(4, L, Stall), (6, R, Pigeons(246)), (7, R, Stall), (9, L, Walker), (12, L, Stall), (14, R, Walker),
            (16, L, Cart), (17, R, Stall), (19, L, Walker), (23, L, Stall), (25, R, Walker), (27, R, Stall),
            (28, L, Pigeons(214)), (31, L, Walker), (33, R, Stall), (35, L, Stall), (37, R, Walker), (39, L, Stall),
            (10, R, Tourist), (22, R, Tourist), (26, L, Tourist), (34, R, Tourist)],
        traffic: &[],
    },
    Day {
        name: "SUNDAY", weather: Weather::Bundles, tell: "SUNDAY BUNDLES: DOUBLE POINTS",
        hazards: &[(6, R, Blader), (9, L, Dog), (14, L, Cart), (17, R, Car), (22, R, Dog), (26, L, Blader),
            (30, R, Cart), (33, L, Car), (36, R, Blader), (39, L, Dog), (5, R, Cyclist), (11, L, Cyclist), (19, R, Cyclist),
            (28, L, Cyclist), (35, R, Cyclist), (13, L, Tourist), (23, L, Tourist), (31, R, Tourist), (38, R, Tourist)],
        traffic: &[26],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_sides_of_the_route_line_up() {
        assert_eq!(ROUTE[0].len(), ROUTE[1].len());
        for row in 0..ROWS {
            let (a, b) = (lot(Side::Left, row).0, lot(Side::Right, row).0);
            for shared in [Lot::Cross, Lot::Bridge, Lot::Palace] {
                assert_eq!(a == shared, b == shared, "row {row}: a side street or bridge on one side only");
            }
        }
    }

    #[test]
    fn every_hazard_stands_where_it_can() {
        for day in &DAYS {
            for &(row, side, hz) in day.hazards {
                let (l, _) = lot(side, row as usize);
                assert!((3..ROWS - 3).contains(&(row as usize)), "{}: row {row} is too near an end", day.name);
                assert!(!matches!(l, Lot::Bridge | Lot::Palace), "{}: {hz:?} on a bridge or the palace square", day.name);
                match hz {
                    Hz::Car => assert_eq!(l, Lot::House { drive: true }, "{}: a car with no driveway at {row}", day.name),
                    Hz::Dog | Hz::Sprinkler => assert!(l.is_house(), "{}: {hz:?} at row {row} has no garden", day.name),
                    _ => {}
                }
            }
            for &row in day.traffic {
                assert_ne!(lot(Side::Left, row as usize).0, Lot::Bridge, "{}: traffic waits on a bridge", day.name);
                // Koebmagergade is for walking (and delivery bikes); Store Kongensgade is closed past the turn
                assert!(!(22..=24).contains(&row) && !(TURN_ROW..NEW_STREET_ROW).contains(&(row as usize)), "{}: traffic at row {row}", day.name);
            }
        }
    }
}
