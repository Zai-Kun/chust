use anyhow::{anyhow, Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

pub struct Stockfish {
    path: String,
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Stockfish {
    /// Creates a new instance of Stockfish and starts the engine process.
    pub fn new(path: &str) -> Result<Self> {
        let mut process = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to start Stockfish process")?;

        let stdin = process.stdin.take().context("Failed to open stdin")?;
        let stdout = BufReader::new(process.stdout.take().context("Failed to open stdout")?);

        Ok(Self {
            path: path.to_string(),
            process,
            stdin,
            stdout,
        })
    }

    /// Sends a command to Stockfish.
    fn send_command(&mut self, command: &str) -> Result<()> {
        writeln!(self.stdin, "{}", command).context("Failed to write command to Stockfish")?;
        self.stdin.flush().context("Failed to flush stdin")?;
        Ok(())
    }

    /// Reads the next line from Stockfish output.
    fn read_line(&mut self) -> Result<String> {
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .context("Failed to read line from Stockfish")?;
        Ok(line.trim().to_string())
    }

    /// Gets the best move for a given FEN and depth.
    pub fn get_best_move(&mut self, fen: &str, depth: u32) -> Result<String> {
        self.send_command(&format!("position fen {}", fen))?;
        self.send_command(&format!("go depth {}", depth))?;

        while let Ok(line) = self.read_line() {
            if line.starts_with("bestmove") {
                return line
                    .split_whitespace()
                    .nth(1)
                    .map(|m| m.to_string())
                    .ok_or_else(|| anyhow!("Failed to parse bestmove from Stockfish output"));
            }
        }
        Err(anyhow!("Stockfish did not return a best move"))
    }

    /// Makes a move from a given FEN and retrieves the new FEN.
    pub fn make_move_and_get_fen(&mut self, fen: &str, move_: &str) -> Result<String> {
        self.send_command(&format!("position fen {} moves {}", fen, move_))?;
        self.send_command("d")?;

        while let Ok(line) = self.read_line() {
            if line.starts_with("Fen: ") {
                return line
                    .split_whitespace()
                    .nth(1)
                    .map(|f| f.to_string())
                    .ok_or_else(|| anyhow!("Failed to parse FEN from Stockfish output"));
            }
        }
        Err(anyhow!("Stockfish did not return a valid FEN"))
    }
}
