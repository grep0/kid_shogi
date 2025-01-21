#![allow(dead_code)]

use std::collections::HashSet;
use string_builder::Builder;
use arrayvec::ArrayVec;

use super::abstract_game as ag;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point(pub usize, pub usize);

fn minus_with_boundaries(a: u8, b:u8, high:u8) -> Option<u8> {
    if a<b { None }
    else if a-b>=high { None }
    else { Some(a-b) }
}

impl Point {
    fn swap_sides(self: &Self) -> Point {
        Point(2-self.0, 3-self.1)
    }

    fn is_within_boundaries(self: &Self) -> bool {
        self.0<3 && self.1<4
    }

    fn to_fen(self: &Self) -> String {
        [(self.0 as u8 + 'a' as u8) as char, (self.1 as u8 + '1' as u8) as char].iter().collect()
    }

    fn from_fen(s: &str) -> Option<Point> {
        if s.len() !=2 { return None }
        let x = minus_with_boundaries(s.chars().nth(0).unwrap() as u8, 'a' as u8, 3);
        let y = minus_with_boundaries(s.chars().nth(1).unwrap() as u8 ,'1' as u8, 4);
        if x.is_none() || y.is_none() { return None }
        Some(Point(x.unwrap() as usize, y.unwrap() as usize))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PieceKind {
    Chicken,
    Elephant,
    Giraffe,
    Hen,
    Lion,
}

fn diff(a: usize, b: usize) -> isize {
    (b as isize)-(a as isize)
}

impl PieceKind {
    const COUNT: usize = 5;
    // Pieces that can be in hand
    const IN_HAND: &[PieceKind] = &[PieceKind::Chicken, PieceKind::Elephant, PieceKind::Giraffe];
    const IN_HAND_COUNT: usize = 3;

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
            PieceKind::Hen => &[(0,-1), (-1,0), (1,0), (-1,1), (0,1), (1,1)],
        };
        deltas.into_iter()
            .map(|&(dx,dy)| (from.0 as isize+dx, from.1 as isize+dy))
            .filter(|&(x,y)| x>=0 && x<3 && y>=0 && y<4)
            .map(|(x,y)| Point(x as usize, y as usize)).collect()
    }

    fn index(self: &Self) -> usize {
        match self {
            PieceKind::Chicken => 0,
            PieceKind::Elephant => 1,
            PieceKind::Giraffe => 2,
            PieceKind::Hen => 3,
            PieceKind::Lion => 4,
        }
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
    pub fn index(self: &Self) -> usize {
        match self {
            Color::Sente => 0,
            Color::Gote => 1,
        }
    }

    pub fn opponent(self: &Self) -> Color {
        match self {
            Color::Sente => Color::Gote,
            Color::Gote => Color::Sente,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cell {
    Piece(PieceKind, Color),
    Empty,
}

pub type Cells = ArrayVec<Cell, 12>;

#[derive(Debug, Clone)]
pub struct Position {
    cells: Cells,
    sente_hand: Vec<PieceKind>,
    gote_hand: Vec<PieceKind>,
    current_player: Color,
}

impl Position {
    const CELL_COUNT: usize = 12;

    fn find_all_pieces(self: &Self, color: Color) -> Vec<(Point, PieceKind)> {
        self.cells.iter().enumerate().filter_map(|(xy, cell)|
            match cell {
                Cell::Piece(pk, c) =>
                    {if *c==color {Some((Position::c_to_p(xy), *pk))} else {None}},
                _ => None
            }
        ).collect()
    }
    
    fn c_to_p(coord: usize) -> Point {
        Point(coord%3, coord/3)
    }
    fn p_to_c(p: &Point) -> usize {
        p.0 + p.1*3
    }
}

fn take_piece(hand: &[PieceKind], pk: PieceKind) -> Option<Vec<PieceKind>> {
    if let Some(index) = hand.into_iter().position(|&x| x==pk) {
        return Some([&hand[..index], &hand[index+1..]].concat())
    }
    return None // TODO
}

#[derive(Debug, Clone, PartialEq)]
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

    fn to_fen(self:&Self) -> String {
        match self {
            Move::Step(from, to) => from.to_fen() + &to.to_fen(),
            Move::Drop(pk, to) => format!("{}*{}", pk.to_fen_char().to_ascii_uppercase(), to.to_fen()),
        }
    }

    fn from_fen(s:&str) -> Option<Move> {
        if s.len()!=4 { return None }
        if s.chars().nth(1).unwrap()=='*' {
            if let Some(pk) = PieceKind::from_fen_char(s.chars().nth(0).unwrap().to_ascii_lowercase()) {
                if let Some(to) = Point::from_fen(&s[2..]) {
                    return Some(Move::Drop(pk, to))
                }
            }
        } else {
            if let Some(from) = Point::from_fen(&s[0..2]) {
                if let Some(to) = Point::from_fen(&s[2..]) {
                    return Some(Move::Step(from, to))
                }
            }
        }
        None
    }
}

impl Position {
    pub fn empty() -> Position {
        return Position {
            cells: Cells::from([Cell::Empty; 12]),
            sente_hand: Vec::new(),
            gote_hand: Vec::new(),
            current_player: Color::Sente
        }
    }

    pub fn swap_sides(self: &Self) -> Position {
        return Position {
            cells: self.cells.iter().rev().map(
                |cell|
                    match cell {
                        Cell::Empty => Cell::Empty,
                        Cell::Piece(pk, c) => Cell::Piece(*pk, c.opponent()),
                    }).collect(),
            sente_hand: self.gote_hand.clone(),
            gote_hand: self.sente_hand.clone(),
            current_player: self.current_player.opponent(),
        }
    }

    fn make_move_sente(self: &Self, mv: &Move) -> Option<Position> {
        match mv {
            Move::Step(from, to) => {
                let from_cell = &self.cells[Position::p_to_c(from)];
                if let Cell::Piece(pk, Color::Sente) = from_cell {
                    if !pk.is_valid_move(from, to) {
                        return None
                    }
                    let to_cell = &self.cells[Position::p_to_c(to)];
                    let maybe_promoted = if to.1==3 { pk.promote() } else {*pk};
                    match to_cell {
                        Cell::Empty => {
                            let mut new_cells = self.cells.clone();
                            new_cells[Position::p_to_c(to)] = Cell::Piece(maybe_promoted, Color::Sente);
                            new_cells[Position::p_to_c(from)] = Cell::Empty;
                            return Some(Position {
                                cells: new_cells,
                                sente_hand: self.sente_hand.clone(),
                                gote_hand: self.gote_hand.clone(),
                                current_player: Color::Gote,
                             })
                        }
                        Cell::Piece(qk, Color::Gote) => {
                            let mut new_cells = self.cells.clone();
                            new_cells[Position::p_to_c(to)] = Cell::Piece(maybe_promoted, Color::Sente);
                            new_cells[Position::p_to_c(from)] = Cell::Empty;
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
                if let Cell::Piece(_,_) = self.cells[Position::p_to_c(to)] {
                    return None  // cannot drop on the head
                }
                if let Some(new_hand) = take_piece(&self.sente_hand, *pk) {
                    let mut new_cells = self.cells.clone();
                    new_cells[Position::p_to_c(to)] = Cell::Piece(*pk, Color::Sente);
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

    pub fn make_move_impl(self: &Self, mv: &Move) -> Option<Position> {
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
        if let Some(xy) = self.cells.iter().position(|v| *v == Cell::Piece(PieceKind::Lion, Color::Sente)) {
            let our_lion_pos = Position::c_to_p(xy);
            if our_lion_pos.1==3 {
                // If any opponent's pieces attacks our lion, nope
                let opp_pieces = self.find_all_pieces(Color::Gote);
                !opp_pieces.into_iter().any(
                    |(pos, pk)|
                        pk.is_valid_move(&pos.swap_sides(), &our_lion_pos.swap_sides()))
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn is_lost(self: &Self) -> bool {
        match self.current_player {
            Color::Gote => { self.is_winning_sente() },
            Color::Sente => { self.swap_sides().is_winning_sente() },
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
        let empty_loc = self.cells.iter().enumerate().filter_map(
            |(xy, &cell)| match cell {
                Cell::Empty => Some(Position::c_to_p(xy)),
                _ => None
            }).collect::<Vec<_>>();
        let drops = uniq_drops.into_iter()
            .flat_map(|&pk| empty_loc.iter()
                .map(move |&p| Move::Drop(pk, p)))
            .filter(|mv|
                match mv {
                    Move::Drop(PieceKind::Chicken, Point(_, 3)) => false,
                    _ => true
                }
            )
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
                match self.cells[Position::p_to_c(&Point(x,y))] {
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
                        pos.cells[Position::p_to_c(&Point(x,y))] = Cell::Piece(pk, Color::Gote);
                        x += 1
                    } else { return None }
                } else if c.is_ascii_uppercase() {
                    if let Some(pk) = PieceKind::from_fen_char(c.to_ascii_lowercase()) {
                        pos.cells[Position::p_to_c(&Point(x,y))] = Cell::Piece(pk, Color::Sente);
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

impl ag::AbstractGame for Position {
    fn possible_moves(self: &Self) -> Vec<String> {
        self.list_possible_moves().into_iter().map(|mv| mv.to_fen()).collect()
    }
    fn make_move(self: &Self, mvstr: &str) -> Option<Self> {
        if let Some(mv) = Move::from_fen(mvstr) {
            self.make_move_impl(&mv).and_then(|pos| {
                Some(pos)
            })
        } else {
            None
        }
    }
    fn to_str(self: &Self) -> String {
        self.to_fen()
    }
    fn is_lost(self: &Self) -> bool {
        (*self).is_lost()
    }
    fn current_player(self: &Self) -> i32 {
        match self.current_player {
            Color::Sente => 0,
            Color::Gote => 1,
        }
    }

    fn pretty_print(self: &Self) -> String {
        let mut lines = Vec::<String>::new();
        for y in (0..4).rev() {
            lines.push(
                (0..3).map(|x| {
                    let pt = Point(x,y);
                    let c = match self.cells[Position::p_to_c(&pt)] {
                        Cell::Empty => '.',
                        Cell::Piece(pt, Color::Sente) => pt.to_fen_char().to_ascii_uppercase(),
                        Cell::Piece(pt, Color::Gote) => pt.to_fen_char(),
                    };
                    c.to_string()
                }).collect::<Vec<String>>().join(" ").to_string())
        }
        lines[0].push_str(" [ ");
        lines[0].extend(self.gote_hand.iter().map(|pt| pt.to_fen_char()));
        lines[0].push_str(" ]");
        lines[3].push_str(" [ ");
        lines[3].extend(self.sente_hand.iter().map(|pt| pt.to_fen_char().to_ascii_uppercase()));
        lines[3].push_str(" ]");
        lines.join("\n")
    }
    
    fn initial() -> Self {
        let cells = Cells::from([
            Cell::Piece(PieceKind::Elephant, Color::Sente),
            Cell::Piece(PieceKind::Lion, Color::Sente),
            Cell::Piece(PieceKind::Giraffe, Color::Sente),
            Cell::Empty,
            Cell::Piece(PieceKind::Chicken, Color::Sente),
            Cell::Empty,
            Cell::Empty,
            Cell::Piece(PieceKind::Chicken, Color::Gote),
            Cell::Empty,
            Cell::Piece(PieceKind::Giraffe, Color::Gote),
            Cell::Piece(PieceKind::Lion, Color::Gote),
            Cell::Piece(PieceKind::Elephant, Color::Gote)]);
        return Position{
            cells: cells,
            sente_hand: Vec::new(),
            gote_hand: Vec::new(),
            current_player: Color::Sente,
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        Position::from_fen(s)
    }
}

impl ag::NeuroPosition for Position {
    fn encode(self: &Self) -> Vec<f64> {
        fn delta(size: usize, pos: usize) -> Vec<f64> {
            let mut d = vec![0.0; size];
            d[pos] = 1.0;
            d
        }
        fn encode_hand(h: &[PieceKind]) -> Vec<f64> {
            let mut v = vec![0.0; PieceKind::IN_HAND_COUNT*2];
            let mut curh = Vec::from(h);
            for &pk in PieceKind::IN_HAND {
                for i in 0..2 {  // max 2 pieces of any kind in hand
                    if let Some(nexth) = take_piece(&curh, pk) {
                        curh = nexth;
                        v[pk.index()*2 + i] = 1.0
                    }
                }
            }
            v
        }
        let mut field: Vec<f64> = self.cells.iter().map(
            |cell| match cell {
                Cell::Empty => vec![0.0; PieceKind::COUNT*2],
                Cell::Piece(pk, c) =>
                        delta(PieceKind::COUNT*2, pk.index() + c.index()*PieceKind::COUNT),
            }.into_iter()).flatten().collect();
        field.append(&mut encode_hand(&self.sente_hand));
        field.append(&mut encode_hand(&self.gote_hand));
        field.append(&mut delta(2, self.current_player as usize));
        field
    }
    fn encode_length() -> usize {
        Position::CELL_COUNT*PieceKind::COUNT*2 + PieceKind::IN_HAND_COUNT*2*2 + 2
    }
}

// Simple evaluator counts the values of pieces on board and in hand 
// c=1, g=e=3, h=5
pub struct SimpleEvaluator {}

impl ag::Evaluator<Position> for SimpleEvaluator {
    fn saturation(&self) -> f64 {
        // Max possible piece advantage = (2*5+2*3+2*3)/2 = 11
        20.0
    }
    fn evaluate_position(&self, pos: &Position) -> f64 {
        if pos.is_lost() {
            return -self.saturation()
        }
        fn piece_value(pk: &PieceKind) -> i32 {
            match pk {
                PieceKind::Chicken => 1,
                PieceKind::Elephant => 3,
                PieceKind::Giraffe => 3,
                PieceKind::Hen => 5,
                PieceKind::Lion => 20,  // fake
            }
        }
        let score_by_board: i32 = pos.cells.iter().map(|cell| match cell {
            Cell::Piece(pk, c) => {
                let pm = piece_value(pk);
                let cm = if *c==Color::Sente {1} else {-1};
                pm*cm
            },
            _ => 0,
        }).sum();
        let score_sente_hand: i32 = pos.sente_hand.iter().map(|pk| piece_value(pk)).sum();
        let score_gote_hand: i32 = pos.gote_hand.iter().map(|pk| piece_value(pk)).sum();
        let mult = if pos.current_player==Color::Sente {1} else {-1};
        ((score_by_board + score_sente_hand - score_gote_hand)*mult) as f64 / 2.0
    }
}

#[cfg(test)]
mod tests;