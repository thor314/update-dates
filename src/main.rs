//! Replace broken links to prior files with existing links
use std::{
    fmt::Display,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::Path,
};

use chrono::Duration;
use chrono::{Local, NaiveDate};

#[derive(PartialEq, Eq, Debug, Clone)]
enum FileFormats {
    Journal(NaiveDate, i32),
    Weekly(NaiveDate, i32),
    Monthly(NaiveDate, i32),
    Quarterly(NaiveDate, i32),
}

impl FileFormats {
    fn new(d: NaiveDate, variant: &str) -> anyhow::Result<Self> {
        match variant {
            "j" => Ok(Self::Journal(
                d,
                d.format("%Y").to_string().parse().unwrap(),
            )),
            "w" => Ok(Self::Weekly(d, d.format("%Y").to_string().parse().unwrap())),
            "m" => Ok(Self::Monthly(
                d,
                d.format("%Y").to_string().parse().unwrap(),
            )),
            "q" => Ok(Self::Quarterly(
                d,
                d.format("%Y").to_string().parse().unwrap(),
            )),
            _ => Err(anyhow::anyhow!("unknown letter")),
        }
    }
    /// Get the last n days, formatted as
    fn get_last_n_days(&self, n: i64) -> Vec<FileFormats> {
        // cut the array of days, per the time-split
        let f = |d: &NaiveDate, m: i64| {
            (0..=(n / m))
                .map(|n| *d - Duration::days(n * m))
                .collect::<Vec<NaiveDate>>()
        };

        match self {
            FileFormats::Journal(d, y) => f(d, 1)
                .into_iter()
                .map(|n| FileFormats::Journal(n, *y))
                .collect(),
            FileFormats::Weekly(d, y) => f(d, 7)
                .into_iter()
                .map(|n| FileFormats::Weekly(n, *y))
                .collect(),
            FileFormats::Monthly(d, y) => f(d, 30)
                .into_iter()
                .map(|n| FileFormats::Monthly(n, *y))
                .collect(), // okay to undershoot
            FileFormats::Quarterly(d, y) => f(d, 91)
                .into_iter()
                .map(|n| FileFormats::Quarterly(n, *y))
                .collect(),
        }
    }

    // fn prev(&self) -> Self {
    //     match self {
    //         FileFormats::Journal(d, y) => Self::Journal(*d - Duration::days(1), *y),
    //         FileFormats::Weekly(d, y) => Self::Weekly(*d - Duration::weeks(1), *y),
    //         FileFormats::Monthly(d, y) => Self::Monthly(*d - Duration::days(31), *y),
    //         FileFormats::Quarterly(d, y) => Self::Quarterly(*d - Duration::days(92), *y),
    //     }
    // }

    fn look_for(&self) -> String {
        match self {
            FileFormats::Journal(_, y) => format!("[[j-{}", y),
            FileFormats::Weekly(_, y) => format!("[[w-{}", y),
            FileFormats::Monthly(_, y) => format!("[[m-{}", y),
            FileFormats::Quarterly(_, y) => format!("[[q-{}", y),
        }
    }
    fn absolute_file_location(&self, location: &str) -> String {
        match self {
            FileFormats::Journal(_, _) => format!("{location}/journal"),
            FileFormats::Weekly(_, _) => format!("{location}/weekly"),
            FileFormats::Monthly(_, _) => format!("{location}/monthly"),
            FileFormats::Quarterly(_, _) => format!("{location}/quarterly"),
        }
    }
}

impl Display for FileFormats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileFormats::Journal(d, _) => write!(f, "j-{}.md", d),
            FileFormats::Weekly(d, year) => {
                let first_of_year = NaiveDate::from_ymd_opt(*year, 1, 1).unwrap();
                let duration = *d - first_of_year;
                let week_n = duration.num_weeks() + 1;
                write!(f, "w-{}-W{}.md", d.format("%Y-%m"), week_n)
            }
            FileFormats::Monthly(d, _) => write!(f, "m-{}.md", d.format("%Y-%m")),
            FileFormats::Quarterly(d, year) => {
                let first_of_year = NaiveDate::from_ymd_opt(*year, 1, 1).unwrap();
                let duration = *d - first_of_year;
                let quarter_n = duration.num_weeks() / 13 + 1;
                write!(f, "q-{year}-Q{quarter_n}.md")
            }
        }
    }
}

fn match_pred(s: char) -> bool {
    s == '[' || s == ']' || s == '#'
}

// run this script once a week to fix backdated journal entries
// eventually, CLI this shit if I want to modify the n days, but 200 is a good pick, more than 2 quarters backwards
fn main() {
    let n_days = 200;
    let location = "/home/thor/note2/periodic";
    // let location = "/home/thor/tmp/periodic"; // testing
    let spans: Vec<Vec<FileFormats>> = ["j", "w", "m", "q"]
        .into_iter()
        .map(|l| {
            FileFormats::new(Local::now().date_naive(), l)
                .unwrap()
                .get_last_n_days(n_days)
        })
        .collect();
    for span in spans {
        // get a list of non-existant files
        let dir = match span.first() {
            Some(ff) => ff.absolute_file_location(location),
            None => panic!("wack"),
        };

        let existent_files: Vec<&FileFormats> = span
            .iter()
            .filter(|day| {
                let path = format!("{}/{}", dir, day);
                let path = Path::new(&path);
                // dbg!(&path, &path.exists());
                path.exists()
            })
            .collect(); // bugcheck: path may be wrong

        // hideous but maybe functional lol
        // for every file in existent files, look for any mention of non-existent files
        // look for any mention of any of the non-existent files in the last `n_days` files. If a mention exists,

        // skip the last file in the list
        let skip_last = if !existent_files.is_empty() {
            existent_files[1..existent_files.len()].to_vec()
        } else {
            existent_files[..].to_vec()
        };
        for file_format in skip_last {
            let filename = format!("{}/{}", dir, file_format);
            let f = File::open(filename.clone()).unwrap();
            let reader = BufReader::new(f);
            let mut lines_to_replace = Vec::new();

            for line in reader.lines().map(|l| l.unwrap()) {
                if line.contains(&file_format.look_for()) {
                    let potential_match: &str = line
                        .split(match_pred)
                        // .inspect(|x| println!("inspecting splits {x}"))
                        .filter(|x| x.contains("2022"))
                        // .inspect(|x| println!("found potential match: {x}"))
                        .take(1)
                        .collect::<Vec<&str>>()[0];

                    if Path::exists(Path::new(&format!("{}/{}.md", dir, potential_match))) {
                        // println!("ok: \n{i}: {line}\nfile: {file_format}");
                        continue;
                    } else {
                        // println!("broken: {potential_match} in {file_format} \n{i}: {line}");
                        // JOHNNY GET YOUR GUN
                        // get the last existing file

                        // replace line with last existing file

                        let last_existing_file = if existent_files.contains(&file_format) {
                            let pos = existent_files
                                .iter()
                                .position(|x| x == &file_format)
                                .unwrap();
                            if pos < existent_files.len() - 1 {
                                Some(existent_files[pos + 1])
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        if let Some(last) = last_existing_file {
                            let last = last.to_string();
                            let replacement_line =
                                line.replace(potential_match, &last[..last.len() - 3]); // drop the `.md`
                            println!("replacing \n{line} \nwith: \n{replacement_line}");
                            lines_to_replace.push((line, replacement_line));
                        }
                    }
                }
            }

            // replace lines (efficient? no, but it works)
            let filestring = fs::read_to_string(&filename).unwrap();
            for (line, replace) in lines_to_replace {
                let updated_filestring = filestring.replace(&line, &replace);
                fs::remove_file(&filename).unwrap();
                File::create(&filename)
                    .unwrap()
                    .write_all(updated_filestring.as_bytes())
                    .unwrap();
            }
        }
    }
}
