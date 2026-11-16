use omnivoice_infer::{
    audio_input::ReferenceAudioProcessor,
    contracts::{GenerationConfig, OutputShaping, ReferenceAudioInput, WaveformInput},
    postprocess::{
        apply_clone_rms_restore, fade_and_pad_audio, finalize_output_audio,
        peak_normalize_auto_voice, remove_silence,
    },
    OmniVoiceError,
};

const SR: u32 = 24_000;
const HOP: usize = 960;
/// -50 dBFS
const THRESHOLD: f32 = 0.003_162_277_7;

fn tone(seconds: f32, amplitude: f32) -> Vec<f32> {
    let n = (seconds * SR as f32) as usize;
    (0..n)
        .map(|i| {
            amplitude * (2.0 * std::f32::consts::PI * 220.0 * (i as f32 + 0.5) / SR as f32).sin()
        })
        .collect()
}

/// Deterministic pseudo-noise at a fixed peak amplitude.
fn noise(seconds: f32, amplitude: f32) -> Vec<f32> {
    let n = (seconds * SR as f32) as usize;
    let mut state: u32 = 0x1234_5678;
    (0..n)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let unit = (state >> 8) as f32 / (1u32 << 24) as f32; // [0, 1)
            amplitude * (unit * 2.0 - 1.0)
        })
        .collect()
}

fn input(samples: Vec<f32>) -> ReferenceAudioInput {
    ReferenceAudioInput::Waveform(WaveformInput::mono(samples, SR))
}

fn processor() -> ReferenceAudioProcessor {
    ReferenceAudioProcessor::new(SR, HOP)
}

fn trailing_zero_run(samples: &[f32]) -> usize {
    samples.iter().rev().take_while(|s| **s == 0.0).count()
}

fn assert_invalid_naming(result: omnivoice_infer::Result<()>, field: &str) {
    match result {
        Err(OmniVoiceError::InvalidRequest(message)) => {
            assert!(
                message.contains(field),
                "message `{message}` does not name {field}"
            )
        }
        other => panic!("expected InvalidRequest naming {field}, got {other:?}"),
    }
}

fn shaping(pad: f32, fade: f32) -> OutputShaping {
    OutputShaping {
        pad_duration: pad,
        fade_duration: fade,
    }
}

// C1
#[test]
fn generation_config_defaults() {
    let config = GenerationConfig::default();
    assert_eq!(config.pad_duration, 0.1);
    assert_eq!(config.fade_duration, 0.1);
    assert_eq!(config.prompt_trailing_silence, None);
}

// C2
#[test]
fn output_pad_adds_exact_zero_samples() {
    let clip = vec![0.5f32; SR as usize];
    let padded =
        finalize_output_audio(clip.clone(), SR, Some(0.1), false, shaping(0.25, 0.0)).unwrap();
    assert_eq!(padded.len(), clip.len() + 2 * 6_000);
    assert!(padded[..6_000].iter().all(|s| *s == 0.0));
    assert!(padded[padded.len() - 6_000..].iter().all(|s| *s == 0.0));
    assert_ne!(padded[6_000], 0.0);
    assert_ne!(padded[padded.len() - 6_001], 0.0);

    let unpadded =
        finalize_output_audio(clip.clone(), SR, Some(0.1), false, shaping(0.0, 0.0)).unwrap();
    assert_eq!(unpadded.len(), clip.len());
    assert_ne!(unpadded[0], 0.0);
    assert_ne!(*unpadded.last().unwrap(), 0.0);
}

// C3
#[test]
fn output_fade_ramps_exact_sample_count() {
    let clip = vec![0.5f32; SR as usize];
    // A constant clip at RMS 0.5 restored to ref_rms 0.5 stays at 0.5 (restore only boosts quiet clips).
    let flat =
        finalize_output_audio(clip.clone(), SR, Some(0.5), false, shaping(0.0, 0.0)).unwrap();
    let level = flat[SR as usize / 2];
    assert!(
        flat.iter().all(|s| *s == level),
        "f = 0 must leave every sample unchanged"
    );

    let faded = finalize_output_audio(clip, SR, Some(0.5), false, shaping(0.0, 0.02)).unwrap();
    let n = faded.len();
    assert_eq!(n, SR as usize);
    // The ramp is numpy-style linspace over 480 samples with both endpoints, as upstream:
    // samples 0..479 are below the level and sample 479 reaches it.
    assert!(
        faded[..479].iter().all(|s| *s < level),
        "fade-in must cover samples 0..479"
    );
    assert_eq!(faded[479], level);
    assert!(
        faded[n - 479..].iter().all(|s| *s < level),
        "fade-out must cover the last 479"
    );
    assert_eq!(faded[n - 480], level);
    assert!(
        faded[480..n - 480].iter().all(|s| *s == level),
        "samples 480..len-480 unchanged"
    );
    assert_eq!(faded[0], 0.0);
    assert_eq!(faded[n - 1], 0.0);
}

// C4
#[test]
fn default_output_shaping_matches_legacy_path() {
    let mut speech = vec![0.0f32; 12_000];
    speech.extend(tone(0.6, 0.05));
    speech.extend(vec![0.0f32; 24_000]);
    speech.extend(tone(0.4, 0.3));
    speech.extend(vec![0.0f32; 6_000]);

    let defaults = GenerationConfig::default().output_shaping();
    for postprocess in [true, false] {
        for ref_rms in [Some(0.2f32), None] {
            let mut legacy = speech.clone();
            if postprocess {
                let trimmed = remove_silence(&legacy, SR, 500, 100, 100);
                if !trimmed.is_empty() {
                    legacy = trimmed;
                }
            }
            legacy = match ref_rms {
                Some(rms) => apply_clone_rms_restore(&legacy, rms),
                None => peak_normalize_auto_voice(&legacy).unwrap(),
            };
            let legacy = fade_and_pad_audio(&legacy, SR, 0.1, 0.1);

            let actual =
                finalize_output_audio(speech.clone(), SR, ref_rms, postprocess, defaults).unwrap();
            assert_eq!(
                actual, legacy,
                "postprocess={postprocess} ref_rms={ref_rms:?}"
            );
        }
    }
}

// C7
#[test]
fn pad_duration_bounds() {
    for ok in [0.0f32, 2.0] {
        let config = GenerationConfig {
            pad_duration: ok,
            ..GenerationConfig::default()
        };
        assert!(
            config.validate().is_ok(),
            "pad_duration {ok} must be accepted"
        );
    }
    for bad in [f32::NAN, -0.01, 2.0001] {
        let config = GenerationConfig {
            pad_duration: bad,
            ..GenerationConfig::default()
        };
        assert_invalid_naming(config.validate(), "pad_duration");
    }
}

// C8
#[test]
fn fade_duration_bounds() {
    for ok in [0.0f32, 1.0] {
        let config = GenerationConfig {
            fade_duration: ok,
            ..GenerationConfig::default()
        };
        assert!(
            config.validate().is_ok(),
            "fade_duration {ok} must be accepted"
        );
    }
    for bad in [f32::INFINITY, -0.01, 1.01] {
        let config = GenerationConfig {
            fade_duration: bad,
            ..GenerationConfig::default()
        };
        assert_invalid_naming(config.validate(), "fade_duration");
    }
}

// C16
#[test]
fn prompt_trailing_silence_bounds() {
    for ok in [None, Some(0.0f32), Some(2.0)] {
        let config = GenerationConfig {
            prompt_trailing_silence: ok,
            ..GenerationConfig::default()
        };
        assert!(
            config.validate().is_ok(),
            "prompt_trailing_silence {ok:?} must be accepted"
        );
    }
    for bad in [Some(2.5f32), Some(-0.1), Some(f32::NAN)] {
        let config = GenerationConfig {
            prompt_trailing_silence: bad,
            ..GenerationConfig::default()
        };
        assert_invalid_naming(config.validate(), "prompt_trailing_silence");
    }
}

fn assert_tail(waveform: &[f32]) {
    let run = trailing_zero_run(waveform);
    assert!(
        (12_000 - (HOP - 1)..=12_000).contains(&run),
        "trailing zero run {run} not in [11041, 12000]"
    );
    let before = waveform[waveform.len() - run - 1];
    assert!(
        before.abs() > THRESHOLD,
        "sample before the run ({before}) is not above -50 dBFS"
    );
    assert_eq!(waveform.len() % HOP, 0);
}

// C10
#[test]
fn trailing_silence_with_preprocess_zero_tail() {
    let mut samples = tone(1.0, 0.5);
    samples.extend(vec![0.0f32; SR as usize]);
    let none = processor()
        .prepare_prompt_audio_with(&input(samples.clone()), Some("hello"), true, None)
        .unwrap();
    let set = processor()
        .prepare_prompt_audio_with(&input(samples), Some("hello"), true, Some(0.5))
        .unwrap();
    assert_tail(&set.waveform);
    // The speech part is the same tone the default path prepares.
    let speech_len = set.waveform.len() - trailing_zero_run(&set.waveform);
    assert_eq!(&set.waveform[..speech_len], &none.waveform[..speech_len]);
}

// C11
#[test]
fn trailing_silence_with_preprocess_strips_noise() {
    let mut samples = tone(1.0, 0.5);
    samples.extend(noise(1.0, 0.001)); // -60 dBFS peak
    let set = processor()
        .prepare_prompt_audio_with(&input(samples), Some("hello"), true, Some(0.5))
        .unwrap();
    assert_tail(&set.waveform);
    let speech_len = set.waveform.len() - trailing_zero_run(&set.waveform);
    // No noise left: everything before the run belongs to the 1 s tone (plus the 100 ms lead keep).
    assert!(
        speech_len <= SR as usize + SR as usize / 10,
        "noise survived: speech_len {speech_len}"
    );
}

// C12
#[test]
fn trailing_silence_without_preprocess() {
    let tone_samples = tone(1.0, 0.5);
    let mut samples = tone_samples.clone();
    samples.extend(vec![0.0f32; 2_400]);
    let none = processor()
        .prepare_prompt_audio_with(&input(samples.clone()), None, false, None)
        .unwrap();
    let set = processor()
        .prepare_prompt_audio_with(&input(samples), None, false, Some(0.5))
        .unwrap();
    assert_tail(&set.waveform);
    let speech_len = set.waveform.len() - trailing_zero_run(&set.waveform);
    assert_eq!(&set.waveform[..speech_len], &none.waveform[..speech_len]);
    assert_eq!(&set.waveform[..speech_len], &tone_samples[..speech_len]);
}

// C13
#[test]
fn trailing_silence_none_is_unchanged() {
    let mut speech = vec![0.0f32; 4_800];
    speech.extend(tone(0.7, 0.04));
    speech.extend(vec![0.0f32; 9_600]);
    speech.extend(tone(0.5, 0.4));
    speech.extend(noise(0.3, 0.001));
    for preprocess in [true, false] {
        for text in [Some("hello there"), None] {
            let old = processor()
                .prepare_prompt_audio(&input(speech.clone()), text, preprocess)
                .unwrap();
            let new = processor()
                .prepare_prompt_audio_with(&input(speech.clone()), text, preprocess, None)
                .unwrap();
            assert_eq!(old, new, "preprocess={preprocess} text={text:?}");
        }
    }
}

// C14
#[test]
fn trailing_silence_keeps_rms_and_text() {
    let mut samples = tone(1.0, 0.05);
    samples.extend(vec![0.0f32; SR as usize]);
    for preprocess in [true, false] {
        let none = processor()
            .prepare_prompt_audio_with(&input(samples.clone()), Some("hello"), preprocess, None)
            .unwrap();
        let set = processor()
            .prepare_prompt_audio_with(
                &input(samples.clone()),
                Some("hello"),
                preprocess,
                Some(0.5),
            )
            .unwrap();
        assert_eq!(set.ref_rms, none.ref_rms, "preprocess={preprocess}");
        assert_eq!(set.ref_text, none.ref_text, "preprocess={preprocess}");
    }
}

// C15
#[test]
fn silent_reference_with_trailing_silence_rejected() {
    let silent = vec![0.0f32; SR as usize];
    match processor().prepare_prompt_audio_with(&input(silent.clone()), None, false, Some(0.5)) {
        Err(OmniVoiceError::InvalidRequest(message)) => {
            assert!(message.contains("empty after silence removal"), "{message}")
        }
        other => panic!("expected InvalidRequest, got {other:?}"),
    }
    assert!(processor()
        .prepare_prompt_audio_with(&input(silent), None, false, None)
        .is_ok());
}

// C17
#[test]
fn trailing_silence_zero_strips_only() {
    let mut samples = tone(1.0, 0.5);
    samples.extend(vec![0.0f32; SR as usize]);
    let set = processor()
        .prepare_prompt_audio_with(&input(samples), None, false, Some(0.0))
        .unwrap();
    let last_loud = set
        .waveform
        .iter()
        .rposition(|s| s.abs() > THRESHOLD)
        .unwrap();
    assert!(
        set.waveform.len() - 1 - last_loud < HOP,
        "{} samples after the last loud sample",
        set.waveform.len() - 1 - last_loud
    );
}
