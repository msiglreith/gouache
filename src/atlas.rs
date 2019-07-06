enum Entry<T> {
    Value(T),
    Empty(usize),
}

struct Row {
    y: u32,
    height: u32,
    glyphs: Vec<Entry<AtlasGlyph>>,
    next_x: u32,
    last_used: usize,
}

impl Row {
    fn new(y: u32, height: u32) -> Row {
        Row { y, height, glyphs: Vec::new(), next_x: 0, last_used: 0 }
    }
}

#[derive(Debug)]
struct AtlasGlyph {
    x: u32,
    width: u32,
    height: u32,
    glyph_id: GlyphId,
}

#[derive(Debug)]
struct Location {
    row: usize,
    glyph: usize,
}

#[derive(Copy, Clone, Debug)]
struct Rect { x: u32, y: u32, w: u32, h: u32 }

pub struct Atlas {
    width: u32,
    height: u32,
    rows: Vec<Entry<Row>>,
    rows_by_height: Vec<usize>,
    next_y: u32,
    map: std::collections::HashMap<GlyphId, Location>,
    counter: usize,
}

impl Atlas {
    pub fn new(width: u32, height: u32) -> Atlas {
        Atlas {
            width,
            height,
            rows: Slab::new(),
            rows_by_height: Vec::new(),
            next_y: 0,
            map: std::collections::HashMap::new(),
            counter: 0,
        }
    }

    pub fn update_counter(&mut self) {
        self.counter += 1;
    }

    pub fn get_cached(&mut self, glyph_id: GlyphId) -> Option<Rect> {
        if let Some(&Location { row, glyph }) = self.map.get(&glyph_id) {
            let row = self.rows.get_mut(row).unwrap();
            row.last_used = self.counter;
            let glyph = row.glyphs.get_mut(glyph).unwrap();
            Some(Rect { x: glyph.x, y: row.y, w: glyph.width, h: glyph.height })
        } else {
            None
        }
    }

    pub fn insert(&mut self, glyph_id: GlyphId, width: u32, height: u32) -> Option<Rect> {
        if width > self.width || height > self.height { return None; }

        let row_index = self.find_row(width, height);
        if row_index.is_none() { return None; }
        let row_index = row_index.unwrap();

        let mut row = self.rows.get_mut(row_index).unwrap();
        let x = row.next_x;
        let glyph = row.glyphs.insert(AtlasGlyph {
            x,
            width,
            height,
            glyph_id,
        });
        row.next_x += width;
        row.last_used = self.counter;

        self.map.insert(glyph_id, Location { row: row_index, glyph });

        Some(Rect { x, y: row.y, w: width, h: height })
    }

    fn find_row(&mut self, width: u32, height: u32) -> Option<usize> {
        let row_height = nearest_pow_2(height);
        // this logic is to ensure that the search finds the first of a sequence of equal elements
        let mut index = self.rows_by_height
            .binary_search_by_key(&(2 * row_height - 1), |row| 2 * self.rows.get(*row).unwrap().height)
            .unwrap_err();
        // try to find an existing tightly sized row
        while index < self.rows_by_height.len() && row_height == self.rows.get(self.rows_by_height[index]).unwrap().height {
            if width <= self.width - self.rows.get(self.rows_by_height[index]).unwrap().next_x {
                return Some(self.rows_by_height[index]);
            }
            index += 1;
        }
        // if there is no exact match, try to add a tightly sized row
        if let Some(new_row_index) = self.try_add_row(index, row_height) {
            return Some(new_row_index);
        }
        // search rows for room starting at tightest fit
        for i in index..self.rows_by_height.len() {
            if width <= self.width - self.rows.get(self.rows_by_height[i]).unwrap().next_x {
                return Some(self.rows_by_height[i]);
            }
        }
        // if we ran out of rows, try to add a new row
        if let Some(row_index) = self.try_add_row(index, row_height) {
            return Some(row_index);
        }
        // need to overwrite some rows
        if let Some(row_index) = self.try_overwrite_rows(row_height) {
            return Some(row_index);
        }
        None
    }

    fn try_add_row(&mut self, index: usize, row_height: u32) -> Option<usize> {
        if row_height <= self.height - self.next_y {
            let row_index = self.rows.insert(Row::new(self.next_y, row_height));
            self.next_y += row_height;
            self.rows_by_height.insert(index, row_index);
            Some(row_index)
        } else {
            None
        }
    }

    fn try_overwrite_rows(&mut self, row_height: u32) -> Option<usize> {
        let mut rows_by_y = self.rows_by_height.clone();
        rows_by_y.sort_by_key(|row| self.rows.get(*row).unwrap().y);
        let mut best_i = 0;
        let mut best_height = 0;
        let mut best_num_rows = 0;
        let mut best_last_used = self.counter as f32 + 1.0;
        for i in 0..rows_by_y.len() {
            let mut num_rows = 0;
            let mut rows_height = 0;
            let mut last_used_sum = 0;
            while row_height > rows_height && i + num_rows < rows_by_y.len() {
                let row = self.rows.get(rows_by_y[i]).unwrap();
                if row.last_used == self.counter { continue; }
                num_rows += 1;
                rows_height += row.height;
                last_used_sum += row.last_used;
            }
            if row_height <= rows_height {
                let last_used_avg = last_used_sum as f32 / num_rows as f32;
                if last_used_avg < best_last_used {
                    best_i = i;
                    best_height = rows_height;
                    best_num_rows = num_rows;
                    best_last_used = last_used_avg;
                }
            }
        }
        if best_height > 0 {
            let y = self.rows.get(rows_by_y[best_i]).unwrap().y;
            for row_index in &rows_by_y[best_i..(best_i + best_num_rows)] {
                self.rows_by_height.remove(*row_index);
                let row = self.rows.remove(*row_index).unwrap();
                for glyph in row.glyphs.iter() {
                    self.map.remove(&glyph.glyph_id);
                }
            }
            let row_index = self.add_row(Row::new(y, row_height));
            if best_height > row_height {
                self.add_row(Row::new(y + row_height, best_height - row_height));
            }
            Some(row_index)
        } else {
            None
        }
    }

    fn add_row(&mut self, row: Row) -> usize {
        let height = row.height;
        let row_index = self.rows.insert(row);
        let index = self.rows_by_height
            .binary_search_by_key(&height, |row| self.rows.get(*row).unwrap().height)
            .unwrap_or_else(|i| i);
        self.rows_by_height.insert(index, row_index);
        row_index
    }
}

fn nearest_pow_2(mut x: u32) -> u32 {
    x -= 1;
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x |= x >> 8;
    x |= x >> 16;
    x + 1
}
