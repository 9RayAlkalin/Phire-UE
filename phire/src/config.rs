use bitflags::bitflags;
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Debug)]
    #[serde(transparent)]
    pub struct Mods: i32 {
        const AUTOPLAY = 1;
        const FLIP_X = 2;
        const FADE_OUT = 4;
        const FULL_SCREEN_JUDGE = 8;
    }
}

#[derive(Clone, Deserialize, Serialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ChallengeModeColor {
    White,
    Green,
    Blue,
    Red,
    Golden,
    #[default] 
    Rainbow,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(rename = "adjust_time_new")]
    pub adjust_time: bool,
    pub aggressive: bool,
    pub aspect_ratio: Option<f32>,
    pub audio_buffer_size: Option<u32>,
    pub audio_compatibility: bool,
    pub challenge_color: ChallengeModeColor,
    pub challenge_rank: u32,
    pub chart_debug_line: f32,
    pub chart_debug_note: f32,
    pub chart_ratio: f32,
    pub all_good: bool,
    pub all_bad: bool,
    pub double_click_to_pause: bool,
    pub fxaa: bool,
    pub interactive: bool,
    pub note_scale: f32,
    pub mods: Mods,
    pub mp_enabled: bool,
    pub mp_address: String,
    pub offline_mode: bool,
    pub offset: f32,
    pub particle: bool,
    pub player_name: String,
    pub player_rks: f32,
    pub res_pack_path: Option<String>,
    pub sample_count: u32,
    pub show_acc: bool,
    pub speed: f32,
    pub touch_debug: bool,
    pub volume_music: f32,
    pub volume_sfx: f32,
    pub volume_bgm: f32,
    pub watermark: String,
    pub roman: bool,
    pub chinese: bool,
    pub combo: String,
    pub difficulty: String,
    pub disable_loading: bool,

    // for compatibility
    pub autoplay: Option<bool>,

    pub disable_audio: bool,
    pub judge_offset: f32,

    pub render_line: bool,
    pub render_line_extra: bool,
    pub render_note: bool,
    pub render_double_hint: bool,
    pub render_ui_pause: bool,
    pub render_ui_name: bool,
    pub render_ui_level: bool,
    pub render_ui_score: bool,
    pub render_ui_combo: bool,
    pub render_ui_bar: bool,
    pub render_bg: bool,
    pub render_bg_dim: bool,
    pub render_extra: bool,
    pub bg_blurriness: f32,

    pub max_particles: usize,

    pub fade: f32,
    pub alpha_tint: bool, // note.alpha <=0.5 blue, note.alpha >0.5 red

    pub rotation_mode: bool,
    pub rotation_flat_mode: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            adjust_time: false,
            aggressive: false,
            aspect_ratio: None,
            audio_buffer_size: None,
            audio_compatibility: false,
            challenge_color: ChallengeModeColor::Rainbow,
            challenge_rank: 45,
            chart_debug_line: 0.0,
            chart_debug_note: 0.0,
            chart_ratio: 1.0,
            all_good: false,
            all_bad: false,
            double_click_to_pause: true,
            fxaa: false,
            interactive: true,
            mods: Mods::default(),
            mp_address: "mp2.phira.cn:12345".to_owned(),
            mp_enabled: false,
            note_scale: 1.0,
            offline_mode: false,
            offset: 0.0,
            particle: true,
            player_name: "Guest".to_string(),
            player_rks: 15.,
            res_pack_path: None,
            sample_count: 1,
            show_acc: false,
            speed: 1.0,
            touch_debug: false,
            volume_music: 1.0,
            volume_sfx: 0.0,
            volume_bgm: 1.0,
            watermark: "".to_string(),
            roman: false,
            chinese: false,
            combo: "COMBO".to_string(),
            difficulty: "".to_string(),
            disable_loading: false,

            autoplay: None,

            disable_audio: false,
            judge_offset: 0.,

            render_line: true,
            render_line_extra: true,
            render_note: true,
            render_double_hint: true,
            render_ui_pause: true,
            render_ui_name: true,
            render_ui_level: true,
            render_ui_score: true,
            render_ui_combo: true,
            render_ui_bar: true,
            render_bg: true,
            render_bg_dim: true,
            render_extra: true,
            bg_blurriness: 80.,

            max_particles: 20000,

            fade: 0.,
            alpha_tint: false,

            rotation_mode: false,
            rotation_flat_mode: false,
        }
    }
}

impl Config {
    pub fn init(&mut self) {
        if let Some(flag) = self.autoplay {
            self.mods.set(Mods::AUTOPLAY, flag);
        }
    }

    #[inline]
    pub fn has_mod(&self, m: Mods) -> bool {
        self.mods.contains(m)
    }

    #[inline]
    pub fn autoplay(&self) -> bool {
        self.has_mod(Mods::AUTOPLAY)
    }

    #[inline]
    pub fn flip_x(&self) -> bool {
        self.has_mod(Mods::FLIP_X)
    }

    #[inline]
    pub fn full_scrrn_judge(&self) -> bool {
        self.has_mod(Mods::FULL_SCREEN_JUDGE)
    }
}
