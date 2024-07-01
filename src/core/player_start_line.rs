use bevy::prelude::*;

#[derive(Default)]
pub struct PlayerStart {
    pub translation: Vec3,
    pub rotation: Quat
}

#[derive(Resource)]
pub struct PlayerStartLines {
    start_lines: Vec<Vec<PlayerStart>>,
    next_indices: Vec<usize>
}

impl PlayerStartLines {
    #[inline]
    pub fn new() -> Self {
        Self{
            start_lines: vec![],
            next_indices: vec![]
        }
    }

    #[inline]
    pub fn push_group(&mut self, player_starts: Vec<PlayerStart>) 
    -> usize {
        debug_assert!(self.start_lines.len() == self.next_indices.len());
        self.start_lines.push(player_starts);
        self.next_indices.push(0);
        self.start_lines.len() - 1
    }

    #[inline]
    pub fn with_group(mut self, player_starts: Vec<PlayerStart>) 
    -> Self {
        self.push_group(player_starts);
        self
    }

    #[inline]
    pub fn next(&mut self, group: usize) -> Option<&PlayerStart> {
        debug_assert!(self.start_lines.len() == self.next_indices.len());
        
        let idx = match self.next_indices.get(group) {
            Some(i) => *i,
            None => return None
        };

        // get by group that has index
        let player_starts = &self.start_lines[group];
        self.next_indices[group] = (idx + 1) % player_starts.len();
        player_starts.get(idx)
    }
}
