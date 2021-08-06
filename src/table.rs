
pub struct Table {
    pub headers: Vec<String>,
    pub lines: Vec<Vec<String>>,
}

impl Table {
    pub fn new<'a, S: 'a, T, U>(data: T, headers: &[U]) -> Self
    where
        T: IntoIterator<Item = &'a [S]>,
        S: AsRef<str>,
        U: AsRef<str>,
    {
        let headers: Vec<String> = headers.iter().map(|e| e.as_ref().to_string()).collect();
        let lines: Vec<Vec<String>> = data
            .into_iter()
            .map(|e| e.iter().map(|e2| e2.as_ref().to_string()).collect())
            .collect();
        Table { headers, lines }
    }

    pub fn insert_column(&mut self, index: usize, header: &str, column: &[impl ToString]) {
        self.headers.insert(index, header.to_string());
        self.lines
            .iter_mut()
            .zip(column.iter())
            .for_each(|(line, entry)| line.insert(index, entry.to_string()));
    }

    // fn iter_test(&self) {
    //     let columns :Vec<String> = self.lines.iter().map(|lin| lin[0].clone()).collect();
    // }

    pub fn print(&self) {
        let mut column_widths = vec![0usize; self.headers.len()];
        for row in self.lines.iter() {
            for (c, value) in row.iter().enumerate() {
                if value.len() > column_widths[c] {
                    column_widths[c] = value.len();
                }
            }
        }
        for (c, header) in self.headers.iter().enumerate() {
            if header.len() > column_widths[c] {
                column_widths[c] = header.len();
            }
        }
        let mut table = String::new();
        for (c, value) in self.headers.iter().enumerate() {
            table.push_str(&format!("{:>1$}", value, column_widths[c] + 2));
        }
        table.push('\n');
        let table_width = column_widths.iter().sum::<usize>() + column_widths.len() * 2;
        for _ in 0..table_width {
            table.push('-');
        }
        table.push('\n');
        for row in self.lines.iter() {
            for (c, value) in row.iter().enumerate() {
                table.push_str(&format!("{:>1$}", value, column_widths[c] + 2));
            }
            table.push('\n');
        }
        println!("{}", table);
    }
}