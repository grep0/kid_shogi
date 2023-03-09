mod rules;

fn main() {
    let pos = rules::Position::initial();
    println!("{:?}", pos);
    println!("{}", pos.to_fen());
    let mv1 = rules::Move::Step(rules::Point(1,1), rules::Point(1,2));
    let pos1 = pos.make_move(&mv1).unwrap();
    println!("{:?}", pos1);
    println!("{}", pos1.to_fen());
    let mv2 = rules::Move::Step(rules::Point(2,3), rules::Point(1,2));
    let pos2 = pos1.make_move(&mv2).unwrap();
    println!("{:?}", pos2);
    println!("{}", pos2.to_fen());
    let mv3 = rules::Move::Drop(rules::PieceKind::Chicken, rules::Point(1,1));
    let pos3 = pos2.make_move(&mv3).unwrap();
    println!("{:?}", pos3);
    println!("{}", pos3.to_fen());
}
