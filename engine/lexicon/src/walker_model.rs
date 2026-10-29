//! Parameters of the probability model stored in `dictionary.bin` v5.
//!
//! Policy counts belong to the caller. This type prices any effective count
//! on the loaded dictionary's scale, using the writer's half-even rounding.

use crate::LexiconError;

#[derive(Debug, Clone, Copy)]
pub struct WalkerModel {
    alpha: f64,
    denominator: f64,
}

impl WalkerModel {
    pub fn new(tokens: u64, vocabulary: u64, alpha: f64) -> Result<Self, LexiconError> {
        let denominator = tokens as f64 + alpha * vocabulary as f64;
        let unseen_cost = -(alpha / denominator).ln() * 1000.0;
        if vocabulary == 0
            || !alpha.is_finite()
            || alpha <= 0.0
            || !denominator.is_finite()
            || !unseen_cost.is_finite()
            || !(0.0..=f64::from(u16::MAX)).contains(&unseen_cost.round_ties_even())
        {
            return Err(LexiconError::InvalidBinary(
                "dictionary.bin: invalid walker model metadata".into(),
            ));
        }
        Ok(Self { alpha, denominator })
    }

    /// Quantized −ln p in milli-nats, matching `WalkerModel.cost_of_count`
    /// in the dictionary writer. An effective user count may exceed a tiny
    /// fixture corpus; its probability is capped at 1 (cost 0).
    pub fn cost_of_count(&self, count: u32) -> u16 {
        let probability = ((f64::from(count) + self.alpha) / self.denominator).min(1.0);
        (-probability.ln() * 1000.0).round_ties_even() as u16
    }
}

#[cfg(test)]
mod tests {
    use super::WalkerModel;

    #[test]
    fn cost_tracks_each_model_parameter() {
        // Independently calculated from −ln((count + α)/(N + αV)) × 1000.
        for (n, v, alpha, expected) in [
            (3_627_713, 167_490, 10.0, 7_878),
            (3_627_713, 167_922, 10.0, 7_879),
            (7_255_426, 167_922, 10.0, 8_400),
            (3_627_713, 167_922, 20.0, 8_149),
        ] {
            assert_eq!(
                WalkerModel::new(n, v, alpha).unwrap().cost_of_count(2_000),
                expected
            );
        }
    }

    #[test]
    fn unseen_and_certain_words_use_the_same_scale() {
        let model = WalkerModel::new(15, 4, 1.0).unwrap();
        assert_eq!(model.cost_of_count(10), 547);
        assert_eq!(model.cost_of_count(5), 1_153);
        assert_eq!(model.cost_of_count(0), 2_944);
        assert_eq!(WalkerModel::new(0, 1, 1.0).unwrap().cost_of_count(0), 0);
        assert_eq!(model.cost_of_count(2_000), 0);
    }

    #[test]
    fn malformed_metadata_is_rejected() {
        for (n, v, alpha) in [
            (1, 0, 10.0),
            (1, 1, 0.0),
            (1, 1, -1.0),
            (1, 1, f64::NAN),
            (1, 1, f64::INFINITY),
            (1, 2, f64::MAX),
            (u64::MAX, 1, f64::MIN_POSITIVE),
        ] {
            assert!(WalkerModel::new(n, v, alpha).is_err());
        }
    }
}
