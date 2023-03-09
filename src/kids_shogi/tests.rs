use super::*;

#[test]
fn point_swap_sides() {
    assert_eq!(Point(2,3).swap_sides(), Point(0,0));
    assert_eq!(Point(0,0).swap_sides(), Point(2,3));
    assert_eq!(Point(1,1).swap_sides(), Point(1,2));
    assert_eq!(Point(1,2).swap_sides(), Point(1,1));
}

#[test]
fn point_fen() {
    assert_eq!(Point(2,3).to_fen(), "c4");
    assert_eq!(Point(0,0).to_fen(), "a1");
    assert_eq!(Point::from_fen("a1").unwrap(), Point(0,0));
    assert_eq!(Point::from_fen("c4").unwrap(), Point(2,3));
}

#[test]
fn move_fen() {
    let step = Move::Step(Point(0,0), Point(0,1));
    assert_eq!(step.to_fen(), "a1a2");
    assert_eq!(Move::from_fen("a1a2").unwrap(), step);
    let drop = Move::Drop(PieceKind::Chicken, Point(2,1));
    assert_eq!(drop.to_fen(), "C*c2");
    assert_eq!(Move::from_fen("C*c2").unwrap(), drop);
}

#[test]
fn take_piece_success() {
    let pieces = vec!(PieceKind::Chicken, PieceKind::Elephant);
    assert_eq!(take_piece(&pieces, PieceKind::Chicken).unwrap(), vec!(PieceKind::Elephant));
    assert_eq!(take_piece(&pieces, PieceKind::Elephant).unwrap(), vec!(PieceKind::Chicken));
    assert_eq!(take_piece(&pieces, PieceKind::Giraffe), None);
}

#[test]
fn initial_position() {
    let pos = Position::initial();
    assert_eq!(pos.to_fen(), "gle/1c1/1C1/ELG b -");
    let moves = pos.list_possible_moves();
    assert_eq!(moves.len(), 4);  // one c, one g, two l
}

#[test]
fn a_few_moves() {
    let pos = Position::initial();
    let mv1 = Move::Step(Point(1,1), Point(1,2));
    let pos1 = pos.make_move(&mv1).unwrap();
    assert_eq!(pos1.to_fen(), "gle/1C1/3/ELG w C");
    let mv2 = Move::Step(Point(2,3), Point(1,2));
    let pos2 = pos1.make_move(&mv2).unwrap();
    assert_eq!(pos2.to_fen(), "gl1/1e1/3/ELG b Cc");
    let mv3 = Move::Drop(PieceKind::Chicken, Point(1,1));
    let pos3 = pos2.make_move(&mv3).unwrap();
    assert_eq!(pos3.to_fen(), "gl1/1e1/1C1/ELG w c");
}

#[test]
fn pos_from_fen() {
    let fen = "gl1/1e1/3/ELG b Cc";
    let pos = Position::from_fen(fen).unwrap();
    assert_eq!(pos.to_fen(), fen);
}

#[test]
fn chicken_promotion() {
    let pos = Position::from_fen("l2/2C/3/L2 b -").unwrap();
    let mv = Move::Step(Point(2,2), Point(2,3));
    let pos2 = pos.make_move(&mv).unwrap();
    assert_eq!(pos2.to_fen(), "l1H/3/3/L2 w -")
}

#[test]
fn demote_on_capture() {
    let pos = Position::from_fen("l2/2h/2C/L2 b -").unwrap();
    let mv = Move::from_fen("c2c3").unwrap();
    let pos2 = pos.make_move(&mv).unwrap();
    assert_eq!(pos2.to_fen(), "l2/2C/3/L2 w C")
}

#[test]
fn win_sente_on_lion_capture() {
    let pos = Position::from_fen("l2/G2/3/L2 b -").unwrap();
    let mv = Move::from_fen("a3a4").unwrap();
    let pos2 = pos.make_move(&mv).unwrap();
    assert!(pos2.is_lost());
}

#[test]
fn win_on_lion_passed() {
    let pos = Position::from_fen("l2/G1L/3/3 b -").unwrap();
    let mv = Move::from_fen("c3c4").unwrap();
    let pos2 = pos.make_move(&mv).unwrap();
    assert!(pos2.is_lost());
}

#[test]
fn no_win_on_lion_passed_under_attack() {
    let pos = Position::from_fen("lg1/G1L/3/3 b -").unwrap();
    let mv = Move::from_fen("c3c4").unwrap();
    let pos2 = pos.make_move(&mv).unwrap();
    assert!(!pos2.is_lost());
}

#[test]
fn win_gote_on_lion_capture() {
    let pos = Position::from_fen("l2/G2/1e1/L2 w -").unwrap();
    let mv = Move::from_fen("b2a1").unwrap();
    let pos2 = pos.make_move(&mv).unwrap();
    assert!(pos2.is_lost());
}

#[test]
fn possible_moves_with_drops_sente() {
    let pos = Position::from_fen("1l1/ge1/1C1/ELG b C").unwrap();
    let mut moves = pos.list_possible_moves().iter().map(|mv| mv.to_fen()).collect::<Vec<_>>();
    moves.sort();
    let mut expected_moves = vec![
        // giraffe
        "c1c2",
        // lion
        "b1a2", "b1c2",
        // elephant (none)
        // chicken
        "b2b3",
        // drops
        "C*a2", "C*c2", "C*c3", "C*a4", "C*c4",
    ];
    expected_moves.sort();
    assert_eq!(moves, expected_moves);
}

#[test]
fn possible_moves_with_drops_gote() {
    let pos = Position::from_fen("1l1/ge1/1C1/ELG w c").unwrap();
    let mut moves = pos.list_possible_moves().iter().map(|mv| mv.to_fen()).collect::<Vec<_>>();
    moves.sort();
    let mut expected_moves = vec![
        // giraffe
        "a3a2", "a3a4",
        // lion
        "b4a4", "b4c4", "b4c3",
        // elephant
        "b3a2", "b3a4", "b3c2", "b3c4",
        // drops
        "C*a2", "C*a4", "C*c2", "C*c3", "C*c4",
    ];
    expected_moves.sort();
    assert_eq!(moves, expected_moves);
}

#[test]
fn chicken_moves() {
    let pos = Position::from_fen("3/3/1C1/3 b -").unwrap();
    let mut moves = pos.list_possible_moves().iter().map(|mv| mv.to_fen()).collect::<Vec<_>>();
    moves.sort();
    let mut expected_moves = vec![
        "b2b3",
    ];
    expected_moves.sort();
    assert_eq!(moves, expected_moves);
}

#[test]
fn giraffe_moves() {
    let pos = Position::from_fen("3/3/1G1/3 b -").unwrap();
    let mut moves = pos.list_possible_moves().iter().map(|mv| mv.to_fen()).collect::<Vec<_>>();
    moves.sort();
    let mut expected_moves = vec![
        "b2b1", "b2a2", "b2c2", "b2b3",
    ];
    expected_moves.sort();
    assert_eq!(moves, expected_moves);
}

#[test]
fn elephant_moves() {
    let pos = Position::from_fen("3/3/1E1/3 b -").unwrap();
    let mut moves = pos.list_possible_moves().iter().map(|mv| mv.to_fen()).collect::<Vec<_>>();
    moves.sort();
    let mut expected_moves = vec![
        "b2a1", "b2a3", "b2c1", "b2c3",
    ];
    expected_moves.sort();
    assert_eq!(moves, expected_moves);
}

#[test]
fn lion_moves() {
    let pos = Position::from_fen("3/3/1L1/3 b -").unwrap();
    let mut moves = pos.list_possible_moves().iter().map(|mv| mv.to_fen()).collect::<Vec<_>>();
    moves.sort();
    let mut expected_moves = vec![
        "b2a1", "b2a2", "b2a3", "b2b1", "b2b3", "b2c1", "b2c2", "b2c3",
    ];
    expected_moves.sort();
    assert_eq!(moves, expected_moves);
}

#[test]
fn hen_moves() {
    let pos = Position::from_fen("3/3/1H1/3 b -").unwrap();
    let mut moves = pos.list_possible_moves().iter().map(|mv| mv.to_fen()).collect::<Vec<_>>();
    moves.sort();
    let mut expected_moves = vec![
        "b2b1", "b2a2", "b2c2", "b2a3", "b2b3", "b2c3",
    ];
    expected_moves.sort();
    assert_eq!(moves, expected_moves);
}

#[test]
fn invalid_moves() {
    let pos = Position::from_fen("1l1/ge1/1C1/ELG b C").unwrap();
    // from empty
    assert!(pos.make_move(&Move::Step(Point(2,1), Point(2,2))).is_none());
    // from enemy location
    assert!(pos.make_move(&Move::Step(Point(1,3), Point(0,3))).is_none());
    // wrong direction for this piece
    assert!(pos.make_move(&Move::Step(Point(1,1), Point(0,1))).is_none());
    // on top of your own piece
    assert!(pos.make_move(&Move::Step(Point(1,0), Point(1,1))).is_none());
    // drop of absent piece
    assert!(pos.make_move(&Move::Drop(PieceKind::Giraffe, Point(0,1))).is_none());
    // drop on your own piece
    assert!(pos.make_move(&Move::Drop(PieceKind::Chicken, Point(0,0))).is_none());
    // drop on opponent's head
    assert!(pos.make_move(&Move::Drop(PieceKind::Chicken, Point(1,3))).is_none());
}
