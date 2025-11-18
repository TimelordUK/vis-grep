use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// Efficiently read the last N lines from a file without reading the entire file
pub struct FileTailReader;

impl FileTailReader {
    /// Read last N lines from file by seeking from the end
    /// Returns (lines, start_line_number, total_lines)
    pub fn read_last_lines(path: &Path, num_lines: usize) -> std::io::Result<(Vec<String>, usize, usize)> {
        let mut file = File::open(path)?;
        let file_size = file.metadata()?.len();
        
        if file_size == 0 {
            return Ok((vec![], 0, 0));
        }

        // Start with a reasonable buffer size based on typical log line lengths
        // Average log line is ~100-200 bytes, so we start with a buffer that should
        // contain more lines than we need
        let initial_buffer_size = (num_lines * 200).min(file_size as usize);
        
        // Try progressively larger chunks from the end until we have enough lines
        let mut buffer_size = initial_buffer_size;
        let mut lines = Vec::new();
        let mut total_lines_estimate = 0;
        
        loop {
            // Seek to position from end of file
            let seek_pos = file_size.saturating_sub(buffer_size as u64);
            file.seek(SeekFrom::Start(seek_pos))?;
            
            // Read from current position to end
            let mut buffer = vec![0; (file_size - seek_pos) as usize];
            file.read_exact(&mut buffer)?;
            
            // If we're not at the beginning of the file, we might be in the middle of a line
            // Find the first newline to start from a clean line boundary
            let start_offset = if seek_pos > 0 {
                buffer.iter().position(|&b| b == b'\n').unwrap_or(0) + 1
            } else {
                0
            };
            
            // Count lines in this buffer
            lines.clear();
            let buffer_str = String::from_utf8_lossy(&buffer[start_offset..]);
            for line in buffer_str.lines() {
                lines.push(line.to_string());
            }
            
            // If we have enough lines or we've read the whole file, we're done
            if lines.len() >= num_lines || seek_pos == 0 {
                // Trim to exactly num_lines from the end
                if lines.len() > num_lines {
                    lines.drain(0..lines.len() - num_lines);
                }
                
                // Estimate total lines (this is approximate but avoids reading the whole file)
                // We use the average line length from our sample to estimate
                if !lines.is_empty() && seek_pos > 0 {
                    let avg_line_length = buffer_size / lines.len().max(1);
                    total_lines_estimate = (file_size as usize / avg_line_length).max(lines.len());
                } else {
                    total_lines_estimate = lines.len();
                }
                
                break;
            }
            
            // Double the buffer size for next attempt, but don't exceed file size
            buffer_size = (buffer_size * 2).min(file_size as usize);
        }
        
        // Calculate the starting line number (0-indexed)
        let start_line = total_lines_estimate.saturating_sub(lines.len());
        
        Ok((lines, start_line, total_lines_estimate))
    }
    
    /// Read new lines from a position to end of file
    /// This is already efficient as it only reads from the given position
    pub fn read_from_position(path: &Path, from_position: u64) -> std::io::Result<Vec<String>> {
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(from_position))?;
        
        let reader = BufReader::new(file);
        let mut lines = Vec::new();
        
        for line in reader.lines() {
            lines.push(line?);
        }
        
        Ok(lines)
    }
    
    /// Read new lines from a position with a limit
    /// This prevents memory issues when a file grows very quickly
    pub fn read_from_position_limited(path: &Path, from_position: u64, max_lines: usize) -> std::io::Result<(Vec<String>, bool)> {
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(from_position))?;
        
        let reader = BufReader::new(file);
        let mut lines = Vec::new();
        let mut truncated = false;
        
        for line in reader.lines() {
            if lines.len() >= max_lines {
                truncated = true;
                break;
            }
            lines.push(line?);
        }
        
        Ok((lines, truncated))
    }
    
    /// Get file size without reading content
    pub fn get_file_size(path: &Path) -> std::io::Result<u64> {
        Ok(path.metadata()?.len())
    }
    
    /// Estimate total lines in file by sampling
    /// This reads a small sample from the middle of the file to estimate average line length
    pub fn estimate_total_lines(path: &Path) -> std::io::Result<usize> {
        let file_size = path.metadata()?.len();
        
        if file_size == 0 {
            return Ok(0);
        }
        
        // Sample 8KB from the middle of the file
        const SAMPLE_SIZE: usize = 8192;
        let mut file = File::open(path)?;
        
        // Seek to middle of file
        let sample_start = file_size.saturating_sub(SAMPLE_SIZE as u64) / 2;
        file.seek(SeekFrom::Start(sample_start))?;
        
        // Read sample
        let mut buffer = vec![0; SAMPLE_SIZE.min(file_size as usize)];
        let bytes_read = file.read(&mut buffer)?;
        buffer.truncate(bytes_read);
        
        // Count lines in sample
        let line_count = buffer.iter().filter(|&&b| b == b'\n').count().max(1);
        
        // Estimate total lines based on sample
        let avg_line_length = bytes_read / line_count;
        let estimated_lines = (file_size as usize / avg_line_length).max(1);
        
        Ok(estimated_lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_read_last_lines_small_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "line 1").unwrap();
        writeln!(temp_file, "line 2").unwrap();
        writeln!(temp_file, "line 3").unwrap();
        temp_file.flush().unwrap();
        
        let (lines, start, total) = FileTailReader::read_last_lines(temp_file.path(), 2).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "line 2");
        assert_eq!(lines[1], "line 3");
        assert_eq!(start, 1);
        assert!(total >= 3);
    }
    
    #[test]
    fn test_read_last_lines_large_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        
        // Write many lines
        for i in 0..1000 {
            writeln!(temp_file, "This is line number {} with some content", i).unwrap();
        }
        temp_file.flush().unwrap();
        
        let (lines, _start, _total) = FileTailReader::read_last_lines(temp_file.path(), 10).unwrap();
        assert_eq!(lines.len(), 10);
        assert!(lines[9].contains("999"));
        assert!(lines[0].contains("990"));
    }
}