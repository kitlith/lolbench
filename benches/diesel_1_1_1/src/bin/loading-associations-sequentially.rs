extern crate diesel_1_1_1 ; extern crate lolbench_support ; use lolbench_support :: { criterion_from_env , init_logging } ; fn main ( ) { init_logging ( ) ; let mut crit = criterion_from_env ( ) ; diesel_1_1_1 :: loading_associations_sequentially ( & mut crit ) ; } # [ test ] fn run_bench ( ) { use std :: default :: Default ; init_logging ( ) ; let mut crit = Criterion :: default ( ) ; diesel_1_1_1 :: loading_associations_sequentially ( & mut crit ) ; }