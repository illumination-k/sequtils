use anyhow::Result;
use flate2::{write::GzEncoder, Compression};
use std::{path::{PathBuf}, io::{BufWriter, Write}};

use structopt::{clap, StructOpt};

use sequtils::{fastq::{Reader, Writer, Record}, reader::open_with_gz, utils::build_regex};

#[derive(Debug, StructOpt)]
#[structopt(name = "exbc")]
#[structopt(long_version(option_env!("LONG_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))))]
#[structopt(setting(clap::AppSettings::ColoredHelp))]
pub struct Opt {
    #[structopt(long = "read1", short="1")]
    pub read1: PathBuf,
    #[structopt(long = "read2", short="2")]
    pub read2: PathBuf,
    #[structopt(long = "outprefix", short = "o")]
    pub out: Option<String>,
    #[structopt(long = "regex")]
    pub regex: String
}

struct Reporter {
    pub filter_read: usize,
    pub remain_read: usize,
}

impl Reporter {
    fn new() -> Self {
        Self {
            filter_read: 0,
            remain_read: 0
        }
    }
    fn write(&self, out: &str) -> Result<()> 
    {
        let mut writer = BufWriter::new(std::fs::File::create(out)?);
        let content = format!("filter read: {}\nremain read: {}\n", self.filter_read, self.remain_read);
        writer.write_all(content.as_bytes())?;
        Ok(())
    }
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    
    let reader1 = Reader::new(open_with_gz(opt.read1)?);
    let reader2 = Reader::new(open_with_gz(opt.read2)?);
    let regex = build_regex(&opt.regex)?;

    let mut writer1 = if let Some(out) = &opt.out {
        let o = BufWriter::new(std::fs::File::create(out.to_owned() + "_valid_cb_1.fastq.gz")?);
        let gz: Box<dyn Write> = Box::new(GzEncoder::new(o, Compression::default()));
        Writer::new(gz)
    } else {
        let o: Box<dyn Write> = Box::new(BufWriter::new(std::io::stderr()));
        Writer::new(o)
    };

    let mut writer2 = if let Some(out) = &opt.out {
        let o = BufWriter::new(std::fs::File::create(out.to_owned() + "_valid_cb_2.fastq.gz")?);
        let gz: Box<dyn Write> = Box::new(GzEncoder::new(o, Compression::default()));
        Writer::new(gz)
    } else {
        let o: Box<dyn Write> = Box::new(BufWriter::new(std::io::stdout()));
        Writer::new(o)
    };

    let mut reporter = Reporter::new();

    for (r1, r2) in reader1.records().zip(reader2.records()) {
        let (r1, r2) = (r1?, r2?);
        let seq = String::from_utf8(r1.seq().to_vec())?;
        let caps = regex.captures(&seq);

        if let Some(caps) = caps {
            let cb = caps.name("cb");
            let umi = caps.name("umi");

            if cb.is_none() || umi.is_none() {
                continue;
            }

            let cb = cb.unwrap(); let umi = umi.unwrap();

            let seq = cb.as_str().to_string() + umi.as_str();
            let mut qual = r1.qual()[cb.start()..cb.end()].to_vec();
            qual.extend(&r1.qual()[umi.start()..umi.end()]);

            let rec1 = Record::with_attrs(r1.id(), r1.desc(), seq.as_bytes(), &qual);
            
            writer1.write_record(&rec1)?;
            writer2.write_record(&r2)?;

            reporter.remain_read += 1;
        } else {
            reporter.filter_read += 1;
        }
    }

    if let Some(out) = opt.out {
        reporter.write(&(out + "_report.txt"))?;
    }

    Ok(())
}