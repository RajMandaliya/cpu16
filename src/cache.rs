/// cpu16 — L1 Cache Simulation (v0.4.0)
///
/// Architecture: direct-mapped, write-through, 16-line cache.
///
/// Each cache line holds one 16-bit word (2 bytes).
/// A memory address maps to a cache line via:
///
///   line_index = (addr / 2) % NUM_LINES        — which cache slot
///   tag        = (addr / 2) / NUM_LINES         — which block of memory
///
/// On a READ:
///   Hit  → return cached word, increment hits
///   Miss → fetch word from memory, store in cache line, increment misses
///
/// On a WRITE (write-through):
///   Always write to memory immediately.
///   If the written address is currently cached, update the cache line too
///   so reads after writes return the correct value (no stale data).
///
/// Why direct-mapped?
///   With only 4 registers, cpu16 programs have simple access patterns.
///   Direct-mapped is the simplest cache that demonstrates the key concepts:
///   conflict misses (two addresses mapping to the same line), cold misses
///   (first access to any address), and capacity misses (working set > cache).
///
/// Why 16 lines?
///   Small enough that interesting conflict misses occur on real programs
///   (bubble_sort's nested loop will show cache thrashing), large enough
///   that tight inner loops see a meaningful hit rate.
///
/// Stats printed at HALT:
///   total accesses, hits, misses, hit rate %, cold misses vs conflict misses
use crate::memory::Memory;

pub const NUM_LINES: usize = 16;

/// A single cache line.
#[derive(Debug, Clone, Copy, Default)]
struct CacheLine {
    /// Whether this line contains valid data.
    valid: bool,
    /// Tag identifying which memory block is stored here.
    tag: u16,
    /// The cached 16-bit word.
    data: u16,
}

/// Cache statistics collected during execution.
#[derive(Debug, Clone, Copy, Default)]
pub struct CacheStats {
    pub reads: u64,
    pub writes: u64,
    pub hits: u64,
    pub misses: u64,
    /// Misses where the line was previously empty (first access).
    pub cold_misses: u64,
    /// Misses where a valid line was evicted (conflict).
    pub conflict_misses: u64,
}

impl CacheStats {
    pub fn total_accesses(&self) -> u64 {
        self.reads + self.writes
    }

    pub fn hit_rate(&self) -> f64 {
        if self.reads == 0 {
            0.0
        } else {
            self.hits as f64 / self.reads as f64 * 100.0
        }
    }
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "┌─────────────────────────────────────────┐")?;
        writeln!(f, "│         L1 Cache Statistics             │")?;
        writeln!(f, "├─────────────────────────────────────────┤")?;
        writeln!(f, "│  Configuration                          │")?;
        writeln!(f, "│    Lines:        {:>6}  (direct-mapped) │", NUM_LINES)?;
        writeln!(f, "│    Line size:    {:>6}  bytes           │", 2)?;
        writeln!(
            f,
            "│    Total size:   {:>6}  bytes           │",
            NUM_LINES * 2
        )?;
        writeln!(f, "├─────────────────────────────────────────┤")?;
        writeln!(f, "│  Access counts                          │")?;
        writeln!(f, "│    Read accesses:   {:>8}             │", self.reads)?;
        writeln!(f, "│    Write accesses:  {:>8}             │", self.writes)?;
        writeln!(
            f,
            "│    Total accesses:  {:>8}             │",
            self.total_accesses()
        )?;
        writeln!(f, "├─────────────────────────────────────────┤")?;
        writeln!(f, "│  Read performance                       │")?;
        writeln!(f, "│    Hits:            {:>8}             │", self.hits)?;
        writeln!(f, "│    Misses:          {:>8}             │", self.misses)?;
        writeln!(
            f,
            "│    Cold misses:     {:>8}             │",
            self.cold_misses
        )?;
        writeln!(
            f,
            "│    Conflict misses: {:>8}             │",
            self.conflict_misses
        )?;
        writeln!(
            f,
            "│    Hit rate:        {:>7.2}%             │",
            self.hit_rate()
        )?;
        writeln!(f, "└─────────────────────────────────────────┘")
    }
}

/// Direct-mapped write-through L1 cache.
pub struct Cache {
    lines: [CacheLine; NUM_LINES],
    pub stats: CacheStats,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            lines: [CacheLine::default(); NUM_LINES],
            stats: CacheStats::default(),
        }
    }

    /// Decompose a byte address into (line_index, tag).
    ///
    /// We operate on word addresses (addr / 2) since every access is 16-bit.
    /// line_index = word_addr % NUM_LINES  (lower bits)
    /// tag        = word_addr / NUM_LINES  (upper bits)
    fn decompose(addr: u16) -> (usize, u16) {
        let word_addr = addr / 2;
        let line_index = (word_addr as usize) % NUM_LINES;
        let tag = word_addr / NUM_LINES as u16;
        (line_index, tag)
    }

    /// Read a 16-bit word. Returns (word, hit).
    /// On a miss, fetches from memory and installs in cache.
    pub fn read_word(&mut self, addr: u16, mem: &Memory) -> u16 {
        self.stats.reads += 1;
        let (idx, tag) = Self::decompose(addr);
        let line = &mut self.lines[idx];

        if line.valid && line.tag == tag {
            // Cache hit
            self.stats.hits += 1;
            line.data
        } else {
            // Cache miss — classify and fetch
            if !line.valid {
                self.stats.cold_misses += 1;
            } else {
                self.stats.conflict_misses += 1;
            }
            self.stats.misses += 1;

            let word = mem.read_word(addr);
            // Install in cache
            self.lines[idx] = CacheLine {
                valid: true,
                tag,
                data: word,
            };
            word
        }
    }

    /// Write a 16-bit word (write-through: always writes to memory).
    /// If this address is currently in the cache, update it to prevent
    /// stale reads after writes.
    pub fn write_word(&mut self, addr: u16, val: u16, mem: &mut Memory) {
        self.stats.writes += 1;
        // Write-through: always update main memory
        mem.write_word(addr, val);

        // Update cache line if this address is currently cached
        let (idx, tag) = Self::decompose(addr);
        let line = &mut self.lines[idx];
        if line.valid && line.tag == tag {
            line.data = val;
        }
        // If not cached: don't install on write (write-allocate disabled)
        // This is the simpler "no-write-allocate" policy: writes go straight
        // to memory, cache is only populated on reads (misses).
    }

    /// Invalidate a specific address (force a miss on next read).
    /// Useful for testing and for future self-modifying code support.
    pub fn invalidate(&mut self, addr: u16) {
        let (idx, tag) = Self::decompose(addr);
        let line = &mut self.lines[idx];
        if line.valid && line.tag == tag {
            line.valid = false;
        }
    }

    /// Flush the entire cache (invalidate all lines).
    pub fn flush(&mut self) {
        for line in self.lines.iter_mut() {
            line.valid = false;
        }
    }

    /// Return a formatted view of all cache lines (for --debug output).
    pub fn dump(&self) -> String {
        let mut out = String::from("Cache state:\n");
        out.push_str("  Line  Valid  Tag     Data\n");
        out.push_str("  ────  ─────  ──────  ──────\n");
        for (i, line) in self.lines.iter().enumerate() {
            if line.valid {
                out.push_str(&format!(
                    "  {:>4}    yes  0x{:04X}  0x{:04X}\n",
                    i, line.tag, line.data
                ));
            } else {
                out.push_str(&format!("  {:>4}     no  ──────  ──────\n", i));
            }
        }
        out
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}
