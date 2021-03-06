use rand::{Rng, SeedableRng, XorShiftRng};

use rayon;

use std::cmp::max;

pub fn merge_sort<T: Ord + Send + Copy>(v: &mut [T]) {
    let n = v.len();
    let mut buf = Vec::with_capacity(n);
    // We always overwrite the buffer before reading to it, and letting rust
    // initialize it would increase the critical path to O(n).
    unsafe {
        buf.set_len(n);
    }
    rsort(v, &mut buf[..]);
}

// Values from manual tuning gigasort on one machine.
const SORT_CHUNK: usize = 32 * 1024;
const MERGE_CHUNK: usize = 64 * 1024;

// Sort src, possibly making use of identically sized buf.
fn rsort<T: Ord + Send + Copy>(src: &mut [T], buf: &mut [T]) {
    if src.len() <= SORT_CHUNK {
        src.sort();
        return;
    }

    // Sort each half into half of the buffer.
    let mid = src.len() / 2;
    let (bufa, bufb) = buf.split_at_mut(mid);
    {
        let (sa, sb) = src.split_at_mut(mid);
        rayon::join(|| rsort_into(sa, bufa), || rsort_into(sb, bufb));
    }

    // Merge the buffer halves back into the original.
    rmerge(bufa, bufb, src);
}

// Sort src, putting the result into dest.
fn rsort_into<T: Ord + Send + Copy>(src: &mut [T], dest: &mut [T]) {
    let mid = src.len() / 2;
    let (s1, s2) = src.split_at_mut(mid);
    {
        // Sort each half.
        let (d1, d2) = dest.split_at_mut(mid);
        rayon::join(|| rsort(s1, d1), || rsort(s2, d2));
    }

    // Merge the halves into dest.
    rmerge(s1, s2, dest);
}

// Merge sorted inputs a and b, putting result in dest.
//
// Note: `a` and `b` have type `&mut [T]` and not `&[T]` because we do
// not want to require a `T: Sync` bound. Using `&mut` references
// proves to the compiler that we are not sharing `a` and `b` across
// threads and thus we only need a `T: Send` bound.
fn rmerge<T: Ord + Send + Copy>(a: &mut [T], b: &mut [T], dest: &mut [T]) {
    // Swap so a is always longer.
    let (a, b) = if a.len() > b.len() { (a, b) } else { (b, a) };
    if dest.len() <= MERGE_CHUNK {
        seq_merge(a, b, dest);
        return;
    }

    // Find the middle element of the longer list, and
    // use binary search to find its location in the shorter list.
    let ma = a.len() / 2;
    let mb = match b.binary_search(&a[ma]) {
        Ok(i) => i,
        Err(i) => i,
    };

    let (a1, a2) = a.split_at_mut(ma);
    let (b1, b2) = b.split_at_mut(mb);
    let (d1, d2) = dest.split_at_mut(ma + mb);
    rayon::join(|| rmerge(a1, b1, d1), || rmerge(a2, b2, d2));
}

// Merges sorted a and b into sorted dest.
#[inline(never)]
fn seq_merge<T: Ord + Copy>(a: &[T], b: &[T], dest: &mut [T]) {
    if b.is_empty() {
        dest.copy_from_slice(a);
        return;
    }
    let biggest = max(*a.last().unwrap(), *b.last().unwrap());
    let mut ai = a.iter();
    let mut an = *ai.next().unwrap();
    let mut bi = b.iter();
    let mut bn = *bi.next().unwrap();
    for d in dest.iter_mut() {
        if an < bn {
            *d = an;
            an = match ai.next() {
                Some(x) => *x,
                None => biggest,
            }
        } else {
            *d = bn;
            bn = match bi.next() {
                Some(x) => *x,
                None => biggest,
            }
        }
    }
}

#[test]
fn test_merge_sort() {
    let mut v = vec![1; 200_000];
    merge_sort(&mut v[..]);

    let sorted: Vec<u32> = (1..1_000_000).collect();
    let mut v = sorted.clone();
    merge_sort(&mut v[..]);
    assert_eq!(sorted, v);

    v.reverse();
    merge_sort(&mut v[..]);
    assert_eq!(sorted, v);
}

pub fn seq_merge_sort<T: Ord + Copy>(v: &mut [T]) {
    let n = v.len();
    let mut buf = Vec::with_capacity(n);
    // We always overwrite the buffer before reading to it, and we want to
    // duplicate the behavior of parallel sort.
    unsafe {
        buf.set_len(n);
    }
    seq_sort(v, &mut buf[..]);
}

// Sort src, possibly making use of identically sized buf.
fn seq_sort<T: Ord + Copy>(src: &mut [T], buf: &mut [T]) {
    if src.len() <= SORT_CHUNK {
        src.sort();
        return;
    }

    // Sort each half into half of the buffer.
    let mid = src.len() / 2;
    let (bufa, bufb) = buf.split_at_mut(mid);
    {
        let (sa, sb) = src.split_at_mut(mid);
        seq_sort_into(sa, bufa);
        seq_sort_into(sb, bufb);
    }

    // Merge the buffer halves back into the original.
    seq_merge(bufa, bufb, src);
}

// Sort src, putting the result into dest.
fn seq_sort_into<T: Ord + Copy>(src: &mut [T], dest: &mut [T]) {
    let mid = src.len() / 2;
    let (s1, s2) = src.split_at_mut(mid);
    {
        // Sort each half.
        let (d1, d2) = dest.split_at_mut(mid);
        seq_sort(s1, d1);
        seq_sort(s2, d2);
    }

    // Merge the halves into dest.
    seq_merge(s1, s2, dest);
}

pub fn is_sorted<T: Send + Ord>(v: &mut [T]) -> bool {
    let n = v.len();
    if n <= SORT_CHUNK {
        for i in 1..n {
            if v[i - 1] > v[i] {
                return false;
            }
        }
        return true;
    }

    let mid = n / 2;
    if v[mid - 1] > v[mid] {
        return false;
    }
    let (a, b) = v.split_at_mut(mid);
    let (left, right) = rayon::join(|| is_sorted(a), || is_sorted(b));
    return left && right;
}

fn default_vec(n: usize) -> Vec<u32> {
    let mut rng = XorShiftRng::from_seed([0, 1, 2, 3]);
    (0..n).map(|_| rng.next_u32()).collect()
}

pub mod bench;
