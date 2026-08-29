use crate::models::{CloneChartPoint, CloneStatistics};

pub fn clone_statistics(values: &[i64]) -> Option<CloneStatistics> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let len = sorted.len();
    let sum: i64 = sorted.iter().sum();
    let mean = sum as f64 / len as f64;
    let median = if len % 2 == 0 {
        (sorted[len / 2 - 1] + sorted[len / 2]) as f64 / 2.0
    } else {
        sorted[len / 2] as f64
    };
    let variance = sorted
        .iter()
        .map(|value| (*value as f64 - mean).powi(2))
        .sum::<f64>()
        / len as f64;
    let p95_index = (len * 95).div_ceil(100).saturating_sub(1);
    Some(CloneStatistics {
        mean,
        median,
        population_variance: variance,
        population_standard_deviation: variance.sqrt(),
        minimum: sorted[0],
        maximum: sorted[len - 1],
        p95: sorted[p95_index],
    })
}

pub fn total_clone_statistics(points: &[CloneChartPoint]) -> Option<CloneStatistics> {
    clone_statistics(
        &points
            .iter()
            .map(|point| point.total_clones)
            .collect::<Vec<_>>(),
    )
}

pub fn unique_clone_statistics(points: &[CloneChartPoint]) -> Option<CloneStatistics> {
    clone_statistics(
        &points
            .iter()
            .map(|point| point.unique_cloners)
            .collect::<Vec<_>>(),
    )
}

/// Median of an arbitrary `f64` series — used for "median of medians" style comparisons (e.g.
/// each repository's own daily-clone median against the median of every repository's median),
/// where `clone_statistics`'s `i64`-based median doesn't apply.
pub fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let len = sorted.len();
    Some(if len % 2 == 0 {
        (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
    } else {
        sorted[len / 2]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_even_median_population_metrics_and_p95() {
        let result = clone_statistics(&[0, 2, 4, 6]).expect("statistics");
        assert_eq!(result.mean, 3.0);
        assert_eq!(result.median, 3.0);
        assert_eq!(result.population_variance, 5.0);
        assert_eq!(result.population_standard_deviation, 5.0_f64.sqrt());
        assert_eq!(result.p95, 6);
    }

    #[test]
    fn returns_none_for_empty_values() {
        assert!(clone_statistics(&[]).is_none());
    }

    #[test]
    fn calculates_single_and_odd_value_medians() {
        let single = clone_statistics(&[7]).expect("single value statistics");
        assert_eq!(single.mean, 7.0);
        assert_eq!(single.median, 7.0);
        assert_eq!(single.population_variance, 0.0);
        assert_eq!(
            clone_statistics(&[9, 1, 5]).expect("odd statistics").median,
            5.0
        );
    }

    #[test]
    fn nearest_rank_p95_includes_zero_days() {
        let values = [0, 0, 0, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(clone_statistics(&values).expect("statistics").p95, 8);
    }

    #[test]
    fn median_averages_the_two_middle_values_for_even_length() {
        assert_eq!(median(&[8.0, 2.0, 5.0]), Some(5.0));
        assert_eq!(median(&[1.0, 2.0, 8.0, 5.0]), Some(3.5));
    }

    #[test]
    fn median_of_empty_slice_is_none() {
        assert_eq!(median(&[]), None);
    }
}
