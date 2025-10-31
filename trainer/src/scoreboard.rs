use std::{
    io::{BufWriter, Write},
    sync::atomic::Ordering,
    time::Instant,
};

use termion::{clear, cursor};

use crate::player::Epoch;

#[derive(Debug, Clone, PartialEq)]
pub struct ScoreBoard {
    pub match_identifier: String,
    pub p1_identifier: String,
    pub p2_identifier: String,
    pub batch_size: usize,

    pub wins: u32,
    pub draws: u32,
    pub losses: u32,

    prev_w: f32,
    prev_l: f32,
    prev_d: f32,

    pub finished_count: usize,

    pub epoch: Epoch,
    pub start_time: Instant,
    pub now: Instant, //I forgot what this is for
}

impl ScoreBoard {
    pub fn new(m: String, p1: String, p2: String, epoch: Epoch, batch_size: usize) -> Self {
        ScoreBoard {
            match_identifier: m,
            batch_size,
            p1_identifier: p1,
            p2_identifier: p2,
            wins: 0,
            draws: 0,
            losses: 0,
            prev_w: 0.0,
            prev_l: 0.0,
            prev_d: 0.0,
            finished_count: 0,
            epoch,
            start_time: Instant::now(),
            now: Instant::now(),
        }
    }

    pub fn now(&mut self) {
        self.now = Instant::now();
    }

    pub fn update(&mut self) {
        self.finished_count = 0;
        let total = self.wins + self.draws + self.losses;
        self.prev_w = 100.0 * (self.wins as f32) / (total as f32);
        self.prev_l = 100.0 * (self.losses as f32) / (total as f32);
        self.prev_d = 100.0 * (self.draws as f32) / (total as f32);

        self.wins = 0;
        self.draws = 0;
        self.losses = 0;

        //self.invalid_count = 0;
        //self.mse_total = 0.0;
        //self.mse_counter = 0;
        //self.mse_average = 0.0;
        self.now = Instant::now();
    }

    pub fn write_to_buf<T: Write>(&mut self, stream: &mut BufWriter<T>) -> std::io::Result<()> {
        writeln!(stream, "======== {} ========", self.match_identifier)?;
        writeln!(stream, "threads finished: {}", self.finished_count)?;
        let total = self.wins + self.draws + self.losses;
        let title = format!("{} vs {}", self.p1_identifier, self.p2_identifier);
        let info = format!("({} games/batch: {}-epoch, {:.2?})", self.batch_size, self.epoch.0, self.now.elapsed());
        let wdl = format!("wins: {}, draws: {}, losses: {}", self.wins, self.draws, self.losses);
        let stat = format!(
            "[{:.2}+({:.2}):{:.2}+({:.2}):{:.2}+({:.2})]",
            100.0 * (self.wins as f32) / (total as f32),
            (100.0 * (self.wins as f32) / (total as f32)) - self.prev_w,
            100.0 * (self.draws as f32) / (total as f32),
            (100.0 * (self.draws as f32) / (total as f32)) - self.prev_d,
            100.0 * (self.losses as f32) / (total as f32),
            (100.0 * (self.losses as f32) / (total as f32)) - self.prev_l
        );
        writeln!(stream, "{}", title)?;
        writeln!(stream, "{}", info)?;
        writeln!(stream, "{}", wdl)?;
        writeln!(stream, "{}", stat)?;
        stream.flush()?;
        Ok(())
    }

    pub fn write<T: Write>(&mut self, stream: &mut T, (x, y): (u16, u16)) -> std::io::Result<()> {
        for i in 0..6 {
            write!(stream, "{}{}", cursor::Goto(x, y + i), clear::CurrentLine)?;
        }
        write!(stream, "{}======== {} ========\n\r", cursor::Goto(x + 0, y), self.match_identifier)?;
        write!(stream, "threads finished: {}\n\r", self.finished_count)?;
        let total = self.wins + self.draws + self.losses;
        let title = format!("{} vs {}", self.p1_identifier, self.p2_identifier);
        let info = format!("({} games/batch: {}-epoch, {:.2?})", self.batch_size, self.epoch.0, self.now.elapsed());
        let wdl = format!("wins: {}, draws: {}, losses: {}", self.wins, self.draws, self.losses);
        let stat = format!(
            "[{:.2}+({:.2}):{:.2}+({:.2}):{:.2}+({:.2})]",
            100.0 * (self.wins as f32) / (total as f32),
            (100.0 * (self.wins as f32) / (total as f32)) - self.prev_w,
            100.0 * (self.draws as f32) / (total as f32),
            (100.0 * (self.draws as f32) / (total as f32)) - self.prev_d,
            100.0 * (self.losses as f32) / (total as f32),
            (100.0 * (self.losses as f32) / (total as f32)) - self.prev_l
        );
        write!(stream, "{}\n\r", title)?;
        write!(stream, "{}\n\r", info)?;
        write!(stream, "{}\n\r", wdl)?;
        write!(stream, "{}\n\r", stat)?;
        stream.flush()?;
        Ok(())
    }
}
