#![allow(dead_code)]

use std::collections::HashSet;
use string_builder::Builder;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point(pub usize, pub usize);

impl Point {
    fn swap_sides(self: &Self) -> Point {
        Point(2-self.0, 3-self.1)
    }

    fn is_within_boundaries(self: &Self) -> bool {
        self.0<3 && self.1<4
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PieceKind {
    Chicken,
    Giraffe,
    Elephant,
    Hen,
    Lion,
}

fn diff(a: usize, b: usize) -> isize {
    (b as isize)-(a as isize)
}

impl PieceKind {
    pub fn promote(self: &Self) -> PieceKind {
        match self {
            PieceKind::Chicken => PieceKind::Hen,
            _ => self.clone(),
        }
    }

    pub fn demote(self: &Self) -> PieceKind {
        match self {
            PieceKind::Hen => PieceKind::Chicken,
            _ => self.clone(),
        }
    }

    pub fn is_valid_move(self: &Self, from: &Point, to: &Point) -> bool {
        // assuming sente
        let dx = diff(from.0, to.0);
        let dy = diff(from.1, to.1);
        match self {
            PieceKind::Chicken => dx==0 && dy==1,
            PieceKind::Elephant => dx.abs()==1 && dy.abs()==1,
            PieceKind::Giraffe => (dx==0 && dy.abs()==1) || (dy==0 && dx.abs()==1),
            PieceKind::Lion => dx.abs()<=1 && dy.abs()<=1 && !(dx==0 && dy==0),
            PieceKind::Hen => (dy==1 && dx.abs()<=1) || (dy==0 && dx.abs()==1) || (dy==-1 && dx==0),
        }
    }

    pub fn list_moves(self: &Self, from: &Point) -> Vec<Point> {
        let deltas : &[(isize,isize)] = match self {
            PieceKind::Chicken => &[(0,1)],
            PieceKind::Elephant => &[(-1,-1), (-1,1), (1,-1), (1,1)],
            PieceKind::Giraffe => &[(-1,0), (0,-1), (0,1), (1,0)],
            PieceKind::Lion => &[(-1,-1), (-1,0), (-1,1), (0,-1), (0,1), (1,-1), (1,0), (1,1)],
            PieceKind::Hen => &[(-1,0), (0,-1), (0,1), (1,-1), (1,0), (1,1)],
        };
        deltas.into_iter()
            .map(|&(dx,dy)| (from.0 as isize+dx, from.1 as isize+dy))
            .filter(|&(x,y)| x>=0 && x<3 && y>=0 && y<4)
            .map(|(x,y)| Point(x as usize, y as usize)).collect()
    }

    fn to_fen_char(self: &Self) -> char {
        match self {
            PieceKind::Chicken => 'c',
            PieceKind::Elephant => 'e',
            PieceKind::Giraffe => 'g',
            PieceKind::Hen => 'h',
            PieceKind::Lion => 'l',
        }
    }

    fn from_fen_char(c: char) -> Option<Self> {
        match c {
            'c' => Some(PieceKind::Chicken),
            'e' => Some(PieceKind::Elephant),
            'g' => Some(PieceKind::Giraffe),
            'h' => Some(PieceKind::Hen),
            'l' => Some(PieceKind::Lion),
            _ => None,
        }
    }

}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    Sente,
    Gote
}

impl Color {
    pub fn opponent(self: &Self) -> Color {
        match self {
            Color::Sente => Color::Gote,
            Color::Gote => Color::Sente,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Cell {
    Piece(PieceKind, Color),
    Empty,
}

#[derive(Debug, Clone)]
pub struct Position {
    cells: Vec<Vec<Cell>>,  // 3x4
    sente_hand: Vec<PieceKind>,
    gote_hand: Vec<PieceKind>,
    current_player: Color,
}

impl Position {
    fn find_all_pieces(self: &Self, color: Color) -> Vec<(Point, PieceKind)> {
        self.cells.iter().enumerate().flat_map(|(x, row)|
            row.iter().enumerate().filter_map(move |(y, cell)|
                match cell {
                    Cell::Piece(pk, c) =>
                        {if *c==color {Some((Point(x, y), pk.to_owned()))} else {None}},
                    _ => None
                }
            )
        ).collect()
    }
}

fn take_piece(hand: &[PieceKind], pk: PieceKind) -> Option<Vec<PieceKind>> {
    if let Some(index) = hand.into_iter().position(|&x| x==pk) {
        return Some([&hand[..index], &hand[index+1..]].concat())
    }
    return None // TODO
}

#[derive(Debug, Clone)]
pub enum Move {
    Step(Point, Point),
    Drop(PieceKind, Point),
}

impl Move {
    fn swap_sides(self:&Self) -> Move {
        match self {
            Move::Step(from, to) => Move::Step(from.swap_sides(), to.swap_sides()),
            Move::Drop(pk,to) => Move::Drop(*pk, to.swap_sides()),
        }
    }
}

impl Position {
    pub fn empty() -> Position {
        return Position {
            cells: vec![vec![Cell::Empty; 4]; 3],
            sente_hand: Vec::new(),
            gote_hand: Vec::new(),
            current_player: Color::Sente
        }
    }

    pub fn initial() -> Position {
        let mut cells = vec![vec![Cell::Empty; 4]; 3];
        cells[0][0] = Cell::Piece(PieceKind::Elephant, Color::Sente);
        cells[1][0] = Cell::Piece(PieceKind::Lion, Color::Sente);
        cells[2][0] = Cell::Piece(PieceKind::Giraffe, Color::Sente);
        cells[1][1] = Cell::Piece(PieceKind::Chicken, Color::Sente);
        cells[1][2] = Cell::Piece(PieceKind::Chicken, Color::Gote);
        cells[2][3] = Cell::Piece(PieceKind::Elephant, Color::Gote);
        cells[1][3] = Cell::Piece(PieceKind::Lion, Color::Gote);
        cells[0][3] = Cell::Piece(PieceKind::Giraffe, Color::Gote);
        return Position{
            cells: cells,
            sente_hand: Vec::new(),
            gote_hand: Vec::new(),
            current_player: Color::Sente,
        }
    }

    pub fn swap_sides(self: &Self) -> Position {
        return Position {
            cells: self.cells.iter().rev().map(
                |row| row.into_iter().rev().map(
                    |cell|
                        match cell {
                            Cell::Empty => Cell::Empty,
                            Cell::Piece(k, c) => Cell::Piece(*k, c.opponent()),
                        }).collect()
                    ).collect(),
            sente_hand: self.gote_hand.clone(),
            gote_hand: self.sente_hand.clone(),
            current_player: self.current_player.opponent(),
        }
    }

    fn make_move_sente(self: &Self, mv: &Move) -> Option<Position> {
        match mv {
            Move::Step(from, to) => {
                let from_cell = &self.cells[from.0][from.1];
                if let Cell::Piece(pk, Color::Sente) = from_cell {
                    if !pk.is_valid_move(from, to) {
                        return None
                    }
                    let to_cell = &self.cells[to.0][to.1];
                    let maybe_promoted = if to.0==3 { pk.promote() } else {*pk};
                    match to_cell {
                        Cell::Empty => {
                            let mut new_cells = self.cells.clone();
                            new_cells[to.0][to.1] = Cell::Piece(maybe_promoted, Color::Sente);
                            new_cells[from.0][from.1] = Cell::Empty;
                            return Some(Position {
                                cells: new_cells,
                                sente_hand: self.sente_hand.clone(),
                                gote_hand: self.gote_hand.clone(),
                                current_player: Color::Gote,
                             })
                        }
                        Cell::Piece(qk, Color::Gote) => {
                            let mut new_cells = self.cells.clone();
                            new_cells[to.0][to.1] = Cell::Piece(maybe_promoted, Color::Sente);
                            new_cells[from.0][from.1] = Cell::Empty;
                            let mut new_hand = self.sente_hand.clone();
                            new_hand.push(qk.demote());
                            return Some(Position {
                                cells: new_cells,
                                sente_hand: new_hand,
                                gote_hand: self.gote_hand.clone(),
                                current_player: Color::Gote,
                            })
                        }
                        _ => return None
                    }
                }
            }
            Move::Drop(pk, to) => {
                if let Cell::Piece(_,_) = self.cells[to.0][to.1] {
                    return None  // cannot drop on the head
                }
                if let Some(new_hand) = take_piece(&self.sente_hand, *pk) {
                    let mut new_cells = self.cells.clone();
                    new_cells[to.0][to.1] = Cell::Piece(*pk, Color::Sente);
                    return Some(Position {
                        cells: new_cells,
                        sente_hand: new_hand,
                        gote_hand: self.gote_hand.clone(),
                        current_player: Color::Gote,
                    })
                } else {
                    // no such piece in hand
                    return None
                }
            }
        }
        return None
    }

    pub fn make_move(self: &Self, mv: &Move) -> Option<Position> {
        match self.current_player {
            Color::Sente => { self.make_move_sente(mv) },
            Color::Gote => { self.swap_sides()
                .make_move_sente(&mv.swap_sides())
                .and_then(|m| Some(m.swap_sides())) },
        }
    }

    fn is_winning_sente(self: &Self) -> bool {
        // Captured opp's lion
        if self.sente_hand.iter().find(|&v| *v==PieceKind::Lion).is_some() {
            return true;
        }
        if let Some(x) = self.cells.iter().position(
                |row| match row[3] {
                    Cell::Piece(PieceKind::Lion, Color::Sente) => true,
                    _ => false}) {
            let opp_position = self.swap_sides();
            let our_lion_pos = Point(x, 3).swap_sides();
            // If any opponent's pieces attacks lion, nope
            let opp_pieces = opp_position.find_all_pieces(Color::Sente);
            !opp_pieces.into_iter().any(|(pos, pk)| pk.is_valid_move(&pos, &our_lion_pos))
        } else {
            false
        }
    }

    pub fn is_winning(self: &Self) -> bool {
        match self.current_player {
            Color::Sente => { self.is_winning_sente() },
            Color::Gote => { self.swap_sides().is_winning_sente() },
        }
    }

    fn list_possible_moves_sente(self: &Self) -> Vec<Move> {
        let our_pieces = self.find_all_pieces(Color::Sente);
        let our_pieces_loc = our_pieces.iter().map(|&(point,_)| point).collect::<HashSet<_>>();
        let steps = our_pieces.iter()
            .flat_map(|&(point,pk)|
                    pk.list_moves(&point).into_iter()
                        .filter(|&p| our_pieces_loc.get(&p).is_none())
                        .map(move |p| Move::Step(point, p)))
            .collect::<Vec<Move>>();
        let uniq_drops = self.sente_hand.iter().collect::<HashSet<_>>();
        let empty_loc = (0..3).into_iter().flat_map(
            |x| (0..4).into_iter().filter(move |&y| match self.cells[x][y] {Cell::Empty => true, _ => false})
                .map(move |y| Point(x,y)))
            .collect::<Vec<_>>();
        let drops = uniq_drops.into_iter()
            .flat_map(|&pk| empty_loc.iter().map(move |&p| Move::Drop(pk, p)))
            .collect::<Vec<_>>();
        [steps, drops].concat()
    }

    pub fn list_possible_moves(self: &Self) -> Vec<Move> {
        match self.current_player {
            Color::Sente => { self.list_possible_moves_sente() },
            Color::Gote => {
                self.swap_sides().list_possible_moves_sente().into_iter().map(|m| m.swap_sides()).collect()
            },
        }
    }

    pub fn to_fen(self: &Self) -> String {
        let mut res = Builder::default();
        for y in (0..4).rev() {
            let mut empties=0;
            if y!=3 {res.append('/')}
            for x in 0..3 {
                match self.cells[x][y] {
                    Cell::Empty => { empties+=1 }
                    Cell::Piece(pk, color) => {
                        if empties>0 { res.append(empties.to_string()) }
                        let ch = pk.to_fen_char();
                        res.append(if color==Color::Sente {ch.to_ascii_uppercase()} else {ch});
                        empties=0
                    }
                }
            }
            if empties>0 { res.append(empties.to_string()) }
        }
        res.append(' ');
        res.append(if self.current_player==Color::Sente {'b'} else {'w'});
        res.append(' ');
        let mut sente_hand = self.sente_hand.clone();
        sente_hand.sort();
        let mut gote_hand = self.gote_hand.clone();
        gote_hand.sort();
        let hand_chars = 
            sente_hand.iter().map(|pk| pk.to_fen_char().to_ascii_uppercase()).chain(
            gote_hand.iter().map(|pk| pk.to_fen_char())).collect::<String>();
        if hand_chars.is_empty() { res.append('-') } else { res.append(hand_chars) }
        return res.string().unwrap();
    }

    pub fn from_fen(fen: &str) -> Option<Self> {
        let pieces = fen.split(' ').collect::<Vec<_>>();
        if pieces.len() != 3 { return None }
        let rows = pieces[0].split('/').collect::<Vec<_>>();
        if rows.len() != 4 { return None }
        let mut pos = Position::empty();
        for y in 0..4 {
            let row = rows[3-y];
            let mut x: usize = 0;
            for c in row.chars() {
                if x>=3 { return None }
                if c.is_digit(10) {
                    x += c.to_digit(10).unwrap() as usize
                } else if c.is_ascii_lowercase() {
                    if let Some(pk) = PieceKind::from_fen_char(c) {
                        pos.cells[x][y] = Cell::Piece(pk, Color::Gote);
                        x += 1
                    } else { return None }
                } else if c.is_ascii_uppercase() {
                    if let Some(pk) = PieceKind::from_fen_char(c.to_ascii_lowercase()) {
                        pos.cells[x][y] = Cell::Piece(pk, Color::Sente);
                        x += 1
                    }
                } else { return None }
            }
        }
        match pieces[1] {
            "b" => pos.current_player = Color::Sente,
            "w" => pos.current_player = Color::Gote,
            _ => return None
        }
        if pieces[2]!="-" {
            for c in pieces[2].chars() {
                if c.is_ascii_lowercase() {
                    if let Some(pk) = PieceKind::from_fen_char(c) {
                        pos.gote_hand.push(pk)
                    } else { return None }
                } else if c.is_ascii_uppercase() {
                    if let Some(pk) = PieceKind::from_fen_char(c.to_ascii_lowercase()) {
                        pos.sente_hand.push(pk)
                    } else { return None }
                } else { return None }
            }
        }
        Some(pos)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_swap_sides() {
        assert_eq!(Point(2,3).swap_sides(), Point(0,0));
        assert_eq!(Point(0,0).swap_sides(), Point(2,3));
        assert_eq!(Point(1,1).swap_sides(), Point(1,2));
        assert_eq!(Point(1,2).swap_sides(), Point(1,1));
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
    fn from_fen() {
        let fen = "gl1/1e1/3/ELG b Cc";
        let pos = Position::from_fen(fen).unwrap();
        println!("{:?}", pos);
        assert_eq!(pos.to_fen(), fen);
    }
}