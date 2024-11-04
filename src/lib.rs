#[derive(Debug)]
pub struct Snowflake {
    datacenter_id: u64,
    worker_id: u64,
    sequence: u64,
    lock: std::sync::Mutex<()>,

    epoch: i64,
    max_sequence: u64,

    worker_id_shift: u64,
    datacenter_id_shift: u64,
    timestamp_left_shift: u64,

    last_timestamp: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct SnowflakeBuilder {
    datacenter_id: u64,
    worker_id: u64,
    sequence: u64,

    epoch: i64,
    datacenter_id_bits: u64,
    worker_id_bits: u64,
    sequence_bits: u64,

    max_datacenter_id: u64,
    max_worker_id: u64,
    max_sequence: u64,

    worker_id_shift: u64,
    datacenter_id_shift: u64,
    timestamp_left_shift: u64,
}

#[derive(Debug)]
pub enum SnowflakeError {
    DatacenterIDOutOfRange((u64, u64, u32)), // config, max, recommended
    WorkerIDOutOfRange((u64, u64, u32)),
}

impl std::fmt::Display for SnowflakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnowflakeError::DatacenterIDOutOfRange(msg) => write!(
                f,
                "datacenter_id ({}) out of range ({}), recommend using {} bits",
                msg.0, msg.1, msg.2
            ),
            SnowflakeError::WorkerIDOutOfRange(msg) => write!(
                f,
                "worker_id ({}) out of range ({}), recommend using {} bits",
                msg.0, msg.1, msg.2
            ),
        }
    }
}

impl SnowflakeBuilder {
    fn new(datacenter_id: u64, worker_id: u64, sequence: u64) -> Self {
        let epoch = 1646875384850;
        let datacenter_id_bits = 5;
        let worker_id_bits = 5;
        let sequence_bits = 12;

        let max_datacenter_id = (-1i64 ^ (-1i64 << datacenter_id_bits)) as u64;
        let max_worker_id = (-1i64 ^ (-1i64 << worker_id_bits)) as u64;
        let max_sequence = (-1i64 ^ (-1i64 << sequence_bits)) as u64;

        Self {
            datacenter_id,
            worker_id,
            sequence,

            epoch,
            datacenter_id_bits,
            worker_id_bits,
            sequence_bits,

            max_datacenter_id,
            max_worker_id,
            max_sequence,

            worker_id_shift: sequence_bits,
            datacenter_id_shift: sequence_bits + worker_id_bits,
            timestamp_left_shift: sequence_bits + worker_id_bits + datacenter_id_bits,
        }
    }

    /// Sets the epoch for the Snowflake instance.
    pub fn with_epoch(mut self, epoch: i64) -> Self {
        self.epoch = epoch;
        return self;
    }

    pub fn with_datacenter_id_bits(mut self, datacenter_id_bits: u64) -> Self {
        self.datacenter_id_bits = datacenter_id_bits;
        return self;
    }

    pub fn with_worker_id_bits(mut self, worker_id_bits: u64) -> Self {
        self.worker_id_bits = worker_id_bits;
        return self;
    }

    pub fn with_sequence_bits(mut self, sequence_bits: u64) -> Self {
        self.sequence_bits = sequence_bits;
        return self;
    }

    /// Builds a new Snowflake instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the [`datacenter_id`] or [`worker_id`] is out of
    /// range, so adjust the number of bits accordingly to your use case. The
    /// default number of bits is 5 for both, which means it can handle up to
    /// 32 datacenters, each with up to 32 workers.
    ///
    /// # Panics
    ///
    /// Only if you don't handle the error.
    ///
    /// [`datacenter_id`]: u64
    /// [`worker_id`]: u64
    pub fn build(&self) -> Result<Snowflake, SnowflakeError> {
        if self.max_datacenter_id < self.datacenter_id {
            let recommended = 64 - self.datacenter_id.leading_zeros();
            return Err(SnowflakeError::DatacenterIDOutOfRange((
                self.datacenter_id,
                self.max_datacenter_id,
                recommended,
            )));
        }

        if self.max_worker_id < self.worker_id {
            let recommended = 64 - self.worker_id.leading_zeros();
            return Err(SnowflakeError::WorkerIDOutOfRange((
                self.worker_id,
                self.max_worker_id,
                recommended,
            )));
        }

        let snowflake = Snowflake {
            datacenter_id: self.datacenter_id,
            worker_id: self.worker_id,
            sequence: self.sequence,
            lock: std::sync::Mutex::new(()),

            epoch: self.epoch,
            max_sequence: self.max_sequence,

            worker_id_shift: self.worker_id_shift,
            datacenter_id_shift: self.datacenter_id_shift,
            timestamp_left_shift: self.timestamp_left_shift,

            last_timestamp: -1,
        };

        return Ok(snowflake);
    }
}

impl Snowflake {
    /// Creates a new Snowflake instance.
    pub fn new(datacenter_id: u64, worker_id: u64, sequence: u64) -> SnowflakeBuilder {
        SnowflakeBuilder::new(datacenter_id, worker_id, sequence)
    }

    fn timestamp(&self) -> i64 {
        return std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
    }

    fn wait_for_next_millis(&self, last_timestamp: i64) -> i64 {
        let mut timestamp = self.timestamp();
        while timestamp <= last_timestamp {
            timestamp = self.timestamp();
        }
        return timestamp;
    }

    /// Generates a new Snowflake ID.
    pub fn generate_id(&mut self) -> u64 {
        let _lock = self.lock.lock().unwrap();
        let mut timestamp = self.timestamp();

        if timestamp < self.last_timestamp {
            panic!("Clock moved backwards. Refusing to generate id");
        }

        if self.last_timestamp == timestamp {
            self.sequence = (self.sequence + 1) & self.max_sequence;
            if self.sequence == 0 {
                timestamp = self.wait_for_next_millis(self.last_timestamp);
            }
        } else {
            self.sequence = 0;
        }

        self.last_timestamp = timestamp;

        let id = (((timestamp - self.epoch) as u64) << self.timestamp_left_shift)
            | (self.datacenter_id << self.datacenter_id_shift)
            | (self.worker_id << self.worker_id_shift)
            | self.sequence;

        return id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        let mut snowflake = Snowflake::new(1, 1, 0).build().unwrap();
        println!("{}", snowflake.generate_id());
    }

    #[test]
    fn test_generate_id_with_builder() {
        assert_eq!(
            Snowflake::new(0, 255, 0).build().unwrap_err().to_string(),
            format!("worker_id (255) out of range (31), recommend using 8 bits")
        );
        assert_eq!(
            Snowflake::new(256, 0, 0).build().unwrap_err().to_string(),
            format!("datacenter_id (256) out of range (31), recommend using 9 bits")
        );
    }
}
