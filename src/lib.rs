use anyhow::{Context, Result as AnyResult};
use boxcars::{CrcCheck, NetworkParse, ParseError, ParserBuilder, Replay};
use either::Either;
use glob::glob;
use std::fs;
use std::path::{Path, PathBuf};

/// Configurable parser that can decode Rocket League replays from various sources.
#[derive(Clone, Debug, Default)]
pub struct ReplayParser {
    crc_check: bool,
    network_parse: bool,
}

/// Replay data paired with the path it originated from.
#[derive(Debug)]
pub struct ParsedReplay {
    pub path: PathBuf,
    pub replay: Replay,
}

impl ReplayParser {
    /// Creates a parser with default options (no forced CRC checks and skips network data).
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables or disables forced CRC validation when parsing a replay.
    pub fn with_crc_check(mut self, crc_check: bool) -> Self {
        self.crc_check = crc_check;
        self
    }

    /// Enables or disables parsing of the network data contained in a replay.
    pub fn with_network_parse(mut self, network_parse: bool) -> Self {
        self.network_parse = network_parse;
        self
    }

    /// Returns whether the parser is configured to force CRC validation.
    pub fn crc_check(&self) -> bool {
        self.crc_check
    }

    /// Returns whether the parser will decode network data while parsing.
    pub fn network_parse(&self) -> bool {
        self.network_parse
    }

    /// Mutably sets whether CRC validation should be forced during parsing.
    pub fn set_crc_check(&mut self, crc_check: bool) {
        self.crc_check = crc_check;
    }

    /// Mutably sets whether the parser should decode network data while parsing.
    pub fn set_network_parse(&mut self, network_parse: bool) {
        self.network_parse = network_parse;
    }

    /// Parses replay bytes according to the configured options.
    pub fn parse_bytes(&self, data: &[u8]) -> Result<Replay, ParseError> {
        ParserBuilder::new(data)
            .with_crc_check(if self.crc_check {
                CrcCheck::Always
            } else {
                CrcCheck::OnError
            })
            .with_network_parse(if self.network_parse {
                NetworkParse::Always
            } else {
                NetworkParse::Never
            })
            .parse()
    }

    /// Opens and parses a replay file on disk.
    pub fn parse_file<P: AsRef<Path>>(&self, path: P) -> AnyResult<Replay> {
        let path = path.as_ref();
        let file = fs::File::open(path)?;
        let mmap = unsafe { memmap2::MmapOptions::new().map(&file) };

        let replay = match mmap {
            Ok(mapped) => self.parse_bytes(&mapped),
            Err(_) => {
                let data = fs::read(path)?;
                self.parse_bytes(&data)
            }
        }?;

        Ok(replay)
    }

    /// Parses a replay from disk, returning it alongside the original path.
    pub fn parse_path(&self, path: PathBuf) -> AnyResult<ParsedReplay> {
        let replay = self.parse_file(&path)?;
        Ok(ParsedReplay { path, replay })
    }
}

/// Expands a directory argument into a stream of replay files.
pub fn expand_directory(dir: &Path) -> impl Iterator<Item = AnyResult<PathBuf>> {
    let dir_glob_fmt = format!("{}/**/*.replay", dir.display());
    let replays =
        glob(&dir_glob_fmt).with_context(|| format!("unable to form glob in {}", dir.display()));

    match replays {
        Err(e) => Either::Left(std::iter::once(Err(e))),
        Ok(replays) => {
            let res = replays.filter_map(|entry| match entry {
                Ok(path) => {
                    if path.is_file() {
                        Some(Ok(path))
                    } else {
                        None
                    }
                }
                Err(err) => Some(Err(err.into())),
            });
            Either::Right(res)
        }
    }
}

/// Expands a list of file or directory arguments into individual replay paths.
pub fn expand_paths(files: &[PathBuf]) -> impl Iterator<Item = AnyResult<PathBuf>> + '_ {
    files.iter().flat_map(|arg_file| {
        let path = Path::new(arg_file);
        if path.is_file() {
            Either::Left(std::iter::once(Ok(path.to_path_buf())))
        } else {
            Either::Right(expand_directory(path))
        }
    })
}
