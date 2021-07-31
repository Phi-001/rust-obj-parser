use std::iter::Peekable;
use std::str::Bytes;

pub struct ObjFile {
    pub position: Vec<Position>,
    pub normal: Vec<Normal>,
    pub texcoord: Vec<Texcoord>,
    pub index: Vec<IndexGroup>,
}

impl ObjFile {
    pub fn new(iter: &mut Peekable<Bytes<'_>>) -> Self {
        let mut obj_file = ObjFile {
            position: vec![],
            normal: vec![],
            texcoord: vec![],
            index: vec![],
        };

        while let Some(byte) = iter.next() {
            match byte {
                b'v' => match iter.next().unwrap() {
                    b' ' => {
                        obj_file.position.push(Position::new(iter));
                    }
                    b'n' => {
                        // Remove the trailing space
                        iter.next();
                        obj_file.normal.push(Normal::new(iter));
                    }
                    b't' => {
                        iter.next();
                        obj_file.texcoord.push(Texcoord::new(iter));
                    }
                    _ => {}
                },
                b'f' => {
                    iter.next();
                    obj_file.index.push(IndexGroup::Index(Face::new(iter)));
                }
                b'g' => {
                    obj_file.index.push(IndexGroup::Group(Group::new(iter)));
                }
                b'\n' => {}
                // # -> comment
                // m -> mtllib
                // u -> usemtl
                // s -> smoothing group
                b'#' | b'm' | b'u' | b's' => {
                    eat_line(iter);
                }
                _ => {}
            }
        }

        obj_file
    }
}

pub struct Position {
    pub position: [f32; 3],
}

impl Position {
    fn new(iter: &mut Peekable<Bytes<'_>>) -> Self {
        let mut position = Position { position: [0.0; 3] };

        let mut index = 0;

        while let Some(byte) = iter.peek() {
            match byte {
                b' ' => {}
                b'-' | b'.' | b'0'..=b'9' => {
                    position.position[index] = parse_f32(iter);
                    index += 1;
                }
                _ => {
                    break;
                }
            }
            iter.next();
        }

        position
    }
}

pub struct Normal {
    pub normal: [f32; 3],
}

impl Normal {
    fn new(iter: &mut Peekable<Bytes<'_>>) -> Self {
        let mut normal = Normal { normal: [0.0; 3] };

        let mut index = 0;

        while let Some(byte) = iter.peek() {
            match byte {
                b' ' => {}
                b'-' | b'.' | b'0'..=b'9' => {
                    normal.normal[index] = parse_f32(iter);
                    index += 1;
                }
                _ => {
                    break;
                }
            }
            iter.next();
        }

        normal
    }
}

pub struct Texcoord {
    pub texcoord: [f32; 2],
}

impl Texcoord {
    fn new(iter: &mut Peekable<Bytes<'_>>) -> Self {
        let mut texcoord = Texcoord { texcoord: [0.0; 2] };

        let mut index = 0;

        while let Some(byte) = iter.peek() {
            match byte {
                b' ' => {}
                b'-' | b'.' | b'0'..=b'9' => {
                    texcoord.texcoord[index] = parse_f32(iter);
                    index += 1;
                }
                _ => {
                    break;
                }
            }
            iter.next();
        }

        texcoord
    }
}

fn parse_f32(iter: &mut Peekable<Bytes<'_>>) -> f32 {
    let mut value = 0;
    let mut sign = 1.0;
    let mut below_dot = 0;
    let mut count = false;

    // Numbers used in obj file are small enough
    // So that all of this will still hold enough precision

    while let Some(byte) = iter.peek() {
        match byte {
            b'-' => {
                sign = -1.0;
            }
            b'0'..=b'9' => {
                value = value * 10 + (byte - 48) as u32;
                if count {
                    below_dot += 1;
                }
            }
            b'.' => {
                count = true;
            }
            _ => {
                break;
            }
        }
        iter.next();
    }

    (value as f64 * sign / 10_f64.powi(below_dot)) as f32
}

pub enum IndexGroup {
    Index(Face),
    Group(Group),
}

pub struct Face {
    pub indices: Vec<Index>,
}

impl Face {
    fn new(iter: &mut Peekable<Bytes<'_>>) -> Self {
        let mut face = Face { indices: vec![] };

        while let Some(byte) = iter.peek() {
            match byte {
                b'1'..=b'9' => {
                    face.indices.push(Index::new(iter));
                }
                b' ' => {}
                _ => {
                    break;
                }
            }
            iter.next();
        }

        face
    }
}

pub struct Index {
    pub position: Option<usize>,
    pub texcoord: Option<usize>,
    pub normal: Option<usize>,
}

impl Index {
    fn new(iter: &mut Peekable<Bytes<'_>>) -> Self {
        let mut index = Index {
            position: None,
            texcoord: None,
            normal: None,
        };

        let mut count = 0;

        while let Some(byte) = iter.peek() {
            match byte {
                b'/' => {
                    count += 1;
                }
                b'1'..=b'9' => match count {
                    0 => {
                        index.position = Some(parse_usize(iter));
                    }
                    1 => {
                        index.texcoord = Some(parse_usize(iter));
                    }
                    2 => {
                        index.normal = Some(parse_usize(iter));
                    }
                    _ => {}
                },
                _ => {
                    break;
                }
            }
            iter.next();
        }

        index
    }
}

#[derive(Clone, Copy)]
pub struct Group {}

impl Group {
    fn new(iter: &mut Peekable<Bytes<'_>>) -> Self {
        eat_line(iter);

        Group {}
    }
}

fn parse_usize(iter: &mut Peekable<Bytes<'_>>) -> usize {
    let mut value = 0;

    while let Some(byte) = iter.peek() {
        match byte {
            b'0'..=b'9' => {
                value = value * 10 + ((byte - 48) as usize);
            }
            _ => {
                break;
            }
        }
        iter.next();
    }

    value
}

fn eat_line(iter: &mut Peekable<Bytes<'_>>) {
    let mut string = String::new();
    while let Some(byte) = iter.peek() {
        string.push(*byte as char);
        if *byte == b'\n' {
            break;
        }
        iter.next();
    }
    println!("{}", string);
}
