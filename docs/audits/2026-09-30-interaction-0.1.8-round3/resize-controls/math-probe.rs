#[derive(Clone,Copy,Debug,PartialEq)] struct RangePair{low:f64,high:f64}
pub(crate) fn normalize_value(value: f64, min: f64, max: f64, step: f64) -> f64 {
    let snapped = min + ((value.clamp(min, max) - min) / step).round() * step;
    snapped.clamp(min, max)
}
fn normalize_pair_values(
    values: RangePair,
    min: f64,
    max: f64,
    step: f64,
    minimum_gap: f64,
) -> RangePair {
    let low = normalize_value(values.low, min, max, step);
    let high = normalize_value(values.high, min, max, step);
    if high - low >= minimum_gap {
        return RangePair { low, high };
    }
    let raise_high = first_step_at_or_above(low + minimum_gap, min, max, step)
        .map(|high| RangePair { low, high });
    let lower_low = last_step_at_or_below(high - minimum_gap, min, max, step)
        .map(|low| RangePair { low, high });
    [raise_high, lower_low]
        .into_iter()
        .flatten()
        .min_by(|left, right| {
            pair_distance(*left, values)
                .partial_cmp(&pair_distance(*right, values))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(RangePair {
            low: min,
            high: max,
        })
}
fn first_step_at_or_above(value: f64, origin: f64, max: f64, step: f64) -> Option<f64> {
    let stepped = origin + ((value - origin) / step).ceil() * step;
    if stepped <= max {
        Some(stepped.max(origin))
    } else {
        (max >= value).then_some(max)
    }
}
fn last_step_at_or_below(value: f64, min: f64, max: f64, step: f64) -> Option<f64> {
    let stepped = min + ((value - min) / step).floor() * step;
    if stepped >= min {
        Some(stepped.min(max))
    } else {
        (min <= value).then_some(min)
    }
}
fn pair_distance(pair: RangePair, source: RangePair) -> f64 {
    (pair.low - source.low).abs() + (pair.high - source.high).abs()
}
fn normalize_constrained_value(
    value: f64,
    origin: f64,
    feasible_min: f64,
    feasible_max: f64,
    global_max: f64,
    step: f64,
) -> f64 {
    let snapped = normalize_value(value, origin, global_max, step);
    let first = origin + ((feasible_min - origin) / step).ceil() * step;
    let last = origin + ((feasible_max - origin) / step).floor() * step;
    if first <= last {
        snapped.clamp(first, last)
    } else {
        value.clamp(feasible_min, feasible_max)
    }
}
fn main(){
 let mut count=0;
 for min in [0.0_f64,0.1,3.5,-20.0] {for span in [1.0_f64,3.7,97.0,100.0]{let max=min+span;
  for step in [0.1_f64,0.3,1.0,3.0,10.0] {if step>span{continue}
   for gap in [0.0,span/10.0,span/3.0,span/2.0,span] {
    for li in 0..=20{for hi in li..=20 {let low=min+span*f64::from(li)/20.0;let high=min+span*f64::from(hi)/20.0;
     if low+gap>high{continue}
     let source=RangePair{low,high};let pair=normalize_pair_values(source,min,max,step,gap);
     assert!(pair.low.is_finite()&&pair.high.is_finite());
     assert!(pair.low>=min-1e-9&&pair.high<=max+1e-9&&pair.high-pair.low+1e-9>=gap,"bounds/gap source={source:?} pair={pair:?} min={min} max={max} step={step} gap={gap}");
     let second=normalize_pair_values(pair,min,max,step,gap);
     assert!((second.low-pair.low).abs()<1e-8&&(second.high-pair.high).abs()<1e-8,"not idempotent source={source:?} pair={pair:?} second={second:?} min={min} max={max} step={step} gap={gap}");
     count+=1;
    }}
   }
  }
 }}
 println!("PASS {count} finite legal source/constraint combinations: bounds, gap and idempotence (1e-9/1e-8 tolerance)");
}
