//! Fila do álbum: ordem (normal ou aleatória), repetir e avançar.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Repeat {
    Off,
    All,
    One,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueueTrack {
    pub path: String,
    pub size: u64,
    pub ext: String,
}

pub struct Queue {
    tracks: Vec<QueueTrack>,
    /// Índices do álbum na ordem de tocar.
    order: Vec<usize>,
    /// Posição atual em `order`.
    pos: usize,
    pub repeat: Repeat,
    shuffle: bool,
}

/// Gerador simples (xorshift64) para embaralhar sem dependência.
fn xorshift(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

impl Queue {
    pub fn new(tracks: Vec<QueueTrack>, start: usize) -> Self {
        let n = tracks.len();
        Self { tracks, order: (0..n).collect(), pos: start.min(n.saturating_sub(1)), repeat: Repeat::All, shuffle: false }
    }

    pub fn tracks(&self) -> &[QueueTrack] {
        &self.tracks
    }

    pub fn order(&self) -> &[usize] {
        &self.order
    }

    fn at(&self, pos: usize) -> Option<(usize, &QueueTrack)> {
        let i = *self.order.get(pos)?;
        Some((i, &self.tracks[i]))
    }

    /// Faixa atual (índice no álbum).
    pub fn current(&self) -> Option<(usize, &QueueTrack)> {
        self.at(self.pos)
    }

    /// Posição seguinte ao fim natural da faixa (respeita repeat).
    fn next_pos(&self, natural_end: bool) -> Option<usize> {
        if natural_end && self.repeat == Repeat::One {
            return Some(self.pos);
        }
        if self.pos + 1 < self.order.len() {
            Some(self.pos + 1)
        } else if self.repeat != Repeat::Off && !self.order.is_empty() {
            Some(0)
        } else {
            None
        }
    }

    /// O que vem depois do fim natural da faixa atual (para emendar sem pausa).
    pub fn peek_next(&self) -> Option<(usize, &QueueTrack)> {
        self.next_pos(true).and_then(|p| self.at(p))
    }

    /// Fim natural da faixa.
    pub fn advance(&mut self) -> Option<(usize, &QueueTrack)> {
        let p = self.next_pos(true)?;
        self.pos = p;
        self.current()
    }

    /// Botão "próxima": repetir faixa não prende.
    pub fn next(&mut self) -> Option<(usize, &QueueTrack)> {
        let p = self.next_pos(false)?;
        self.pos = p;
        self.current()
    }

    pub fn prev(&mut self) -> Option<(usize, &QueueTrack)> {
        self.pos = self.pos.saturating_sub(1);
        self.current()
    }

    /// Vai para a faixa `album_index` do álbum.
    pub fn jump(&mut self, album_index: usize) {
        if let Some(p) = self.order.iter().position(|&i| i == album_index) {
            self.pos = p;
        }
    }

    /// Liga/desliga o aleatório mantendo a faixa atual.
    pub fn set_shuffle(&mut self, on: bool, seed: u64) {
        let Some((cur, _)) = self.current() else { return };
        self.shuffle = on;
        if on {
            let mut rest: Vec<usize> = (0..self.tracks.len()).filter(|&i| i != cur).collect();
            let mut s = seed | 1;
            for i in (1..rest.len()).rev() {
                let j = (xorshift(&mut s) % (i as u64 + 1)) as usize;
                rest.swap(i, j);
            }
            self.order = std::iter::once(cur).chain(rest).collect();
            self.pos = 0;
        } else {
            self.order = (0..self.tracks.len()).collect();
            self.pos = cur;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(n: usize, start: usize) -> Queue {
        Queue::new((0..n).map(|i| QueueTrack { path: format!("{i}.flac"), size: 1, ext: ".flac".into() }).collect(), start)
    }
    fn idx(x: Option<(usize, &QueueTrack)>) -> Option<usize> {
        x.map(|(i, _)| i)
    }

    #[test]
    fn advance_stops_at_end_when_repeat_off() {
        let mut q = q(3, 1);
        q.repeat = Repeat::Off;
        assert_eq!(idx(q.current()), Some(1));
        assert_eq!(idx(q.advance()), Some(2));
        assert_eq!(idx(q.advance()), None);
    }

    #[test]
    fn repeat_all_wraps() {
        let mut q = q(2, 1);
        q.repeat = Repeat::All;
        assert_eq!(idx(q.advance()), Some(0));
    }

    #[test]
    fn repeat_one_repeats_on_advance_but_next_moves() {
        let mut q = q(3, 0);
        q.repeat = Repeat::One;
        assert_eq!(idx(q.advance()), Some(0));
        assert_eq!(idx(q.next()), Some(1));
    }

    #[test]
    fn next_at_end_wraps_only_with_repeat() {
        let mut q = q(2, 1);
        q.repeat = Repeat::Off;
        assert_eq!(idx(q.next()), None);
        q.repeat = Repeat::One;
        assert_eq!(idx(q.next()), Some(0));
    }

    #[test]
    fn prev_at_start_stays() {
        let mut q = q(3, 0);
        assert_eq!(idx(q.prev()), Some(0));
        q.jump(2);
        assert_eq!(idx(q.prev()), Some(1));
    }

    #[test]
    fn shuffle_keeps_current_first_and_is_permutation() {
        let mut a = q(10, 4);
        a.set_shuffle(true, 42);
        let mut b = q(10, 4);
        b.set_shuffle(true, 42);
        assert_eq!(a.order(), b.order());
        assert_eq!(a.order()[0], 4);
        let mut sorted = a.order().to_vec();
        sorted.sort();
        assert_eq!(sorted, (0..10).collect::<Vec<_>>());
        assert_ne!(a.order(), (4..10).chain(0..4).collect::<Vec<_>>().as_slice());
        assert_eq!(idx(a.current()), Some(4));
    }

    #[test]
    fn unshuffle_resumes_album_order_after_current() {
        let mut q = q(5, 0);
        q.set_shuffle(true, 7);
        q.advance();
        let cur = idx(q.current()).unwrap();
        q.set_shuffle(false, 0);
        assert_eq!(idx(q.current()), Some(cur));
        q.repeat = Repeat::Off;
        let next = idx(q.advance());
        assert_eq!(next, if cur + 1 < 5 { Some(cur + 1) } else { None });
    }

    #[test]
    fn peek_next_matches_advance() {
        for r in [Repeat::Off, Repeat::All, Repeat::One] {
            let mut q = q(3, 2);
            q.repeat = r;
            let peek = idx(q.peek_next());
            assert_eq!(peek, idx(q.advance()), "{r:?}");
        }
    }

    #[test]
    fn jump_moves_to_album_index_even_when_shuffled() {
        let mut q = q(6, 0);
        q.set_shuffle(true, 3);
        q.jump(5);
        assert_eq!(idx(q.current()), Some(5));
    }
}
