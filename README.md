# phpp is composer replacement， writed by rust

*Developing! Dont used production*


Support subcommand :

- require
- install
- remove
- clear
- dump-autoload
- search
- config set repo


## Usage
1. clone the repository
```
git clone git@github.com:tu6ge/phpp.git
```

2. Run composer command (install package)
```
cargo run require guzzlehttp/guzzle
```

## Bench

```
hyperfine 'cd run_phpp && ./phpp require guzzlehttp/guzzle' 'cd run_composer && composer require guzzlehttp/guzzle' 
```
1. no file cache
```
Benchmark 1: cd run_phpp && ./phpp require guzzlehttp/guzzle
  Time (mean ± σ):      1.675 s ±  5.043 s    [User: 0.052 s, System: 0.038 s]
  Range (min … max):    0.069 s … 16.028 s    10 runs

Benchmark 2: cd run_composer && composer require guzzlehttp/guzzle
  Time (mean ± σ):      4.608 s ±  0.886 s    [User: 0.428 s, System: 0.090 s]
  Range (min … max):    3.593 s …  6.553 s    10 runs
```
2. have file cache

```
Benchmark 1: cd run_phpp && ./phpp require guzzlehttp/guzzle
  Time (mean ± σ):      80.0 ms ±  24.5 ms    [User: 38.5 ms, System: 27.9 ms]
  Range (min … max):    66.9 ms … 170.7 ms    17 runs

Benchmark 2: cd run_composer && composer require guzzlehttp/guzzle
  Time (mean ± σ):      5.095 s ±  0.869 s    [User: 0.386 s, System: 0.077 s]
  Range (min … max):    4.317 s …  6.688 s    10 runs
```