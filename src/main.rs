mod kids_shogi;
mod abstract_game;
mod strategy;

fn main() {
    let pos = kids_shogi::Position::initial();
    println!("{:?}", pos);
    println!("{}", pos.to_fen());
    let mv1 = kids_shogi::Move::Step(kids_shogi::Point(1,1), kids_shogi::Point(1,2));
    let pos1 = pos.make_move(&mv1).unwrap();
    println!("{:?}", pos1);
    println!("{}", pos1.to_fen());
    let mv2 = kids_shogi::Move::Step(kids_shogi::Point(2,3), kids_shogi::Point(1,2));
    let pos2 = pos1.make_move(&mv2).unwrap();
    println!("{:?}", pos2);
    println!("{}", pos2.to_fen());
    let mv3 = kids_shogi::Move::Drop(kids_shogi::PieceKind::Chicken, kids_shogi::Point(1,1));
    let pos3 = pos2.make_move(&mv3).unwrap();
    println!("{:?}", pos3);
    println!("{}", pos3.to_fen());
}
