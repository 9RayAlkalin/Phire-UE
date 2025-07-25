crate::tl_file!("parser");

use super::{BpmList, Effect, JudgeLine, JudgeLineKind, Matrix, Resource, UIElement, Vector, Video};
use crate::{core::Object, fs::FileSystem, judge::JudgeStatus, scene::show_error, ui::Ui};
use anyhow::{Context, Result};
use macroquad::prelude::*;
use sasa::AudioClip;
use std::{cell::RefCell, collections::HashMap};

#[derive(Default)]
pub struct ChartExtra {
    pub effects: Vec<Effect>,
    pub global_effects: Vec<Effect>,
    pub videos: Vec<Video>,
}

#[derive(Default)]
pub struct ChartSettings {
    pub pe_alpha_extension: bool,
    pub hold_partial_cover: bool,
}

pub type HitSoundMap = HashMap<String, AudioClip>;

pub struct Chart {
    pub offset: f32,
    pub lines: Vec<JudgeLine>,
    pub bpm_list: RefCell<BpmList>,
    pub settings: ChartSettings,
    pub extra: ChartExtra,

    pub order: Vec<usize>,
    pub attach_ui: [Option<usize>; 7],
    pub hitsounds: HitSoundMap,
}

impl Chart {
    pub fn new(offset: f32, lines: Vec<JudgeLine>, bpm_list: BpmList, settings: ChartSettings, extra: ChartExtra, hitsounds: HitSoundMap) -> Self {
        let mut attach_ui = [None; 7];
        let mut order = (0..lines.len())
            .filter(|it| {
                if let Some(element) = lines[*it].attach_ui {
                    attach_ui[element as usize - 1] = Some(*it);
                    false
                } else {
                    true
                }
            })
            .collect::<Vec<_>>();
        order.sort_by_key(|it| (lines[*it].z_index, *it));
        Self {
            offset,
            lines,
            bpm_list: RefCell::new(bpm_list),
            settings,
            extra,

            order,
            attach_ui,
            hitsounds,
        }
    }

    #[inline]
    pub fn with_element<R>(&self, ui: &mut Ui, res: &Resource, element: UIElement, scale_point: Option<(f32, f32)>, rotation_point: Option<(f32, f32)>, f: impl FnOnce(&mut Ui, Color) -> R) -> R {
        if let Some(id) = self.attach_ui[element as usize - 1] {
            let lines = &self.lines;
            let line = &lines[id];
            let obj = &line.object;
            let translation = {
                let mut tr = line.fetch_pos(res, lines);
                tr.y *= -res.aspect_ratio;
                tr.x *= res.aspect_ratio;
                let sc = obj.now_scale_wrt_point(scale_point.map_or_else(|| Vector::default(), |(x, y)| Vector::new(x, y)));
                let ro = Object::new_translation_wrt_point(line.fetch_rotate(res, &lines), rotation_point.map_or_else(|| Vector::default(), |(x, y)| Vector::new(x, y)));
                Matrix::new_translation(&tr) * ro * sc
            };
            let mut color = self.lines[id].color.now_opt().unwrap_or(WHITE);
            color.a *= obj.now_alpha().max(0.); 
            ui.with(translation, |ui| f(ui, color))
        } else {
            f(ui, WHITE)
        }
    }

    pub fn with_element_noscale<R>(&self, ui: &mut Ui, res: &Resource, element: UIElement, ct: Option<(f32, f32)>, f: impl FnOnce(&mut Ui, Color) -> R) -> R {
        if let Some(id) = self.attach_ui[element as usize - 1] {
            let obj = &self.lines[id].object;
            let mut tr = obj.now_translation(res);
            tr.y = -tr.y;
            let mut color = self.lines[id].color.now_opt().unwrap_or(WHITE);
            color.a *= obj.now_alpha().max(0.); 
            let mut scale = obj.now_scale_wrt_point(ct.map_or_else(|| Vector::default(), |(x, y)| Vector::new(x , y)));
            scale.m11 = 1.0;
            ui.with(obj.now_rotation().append_translation(&tr) * scale, |ui| f(ui, color))
        } else {
            f(ui, WHITE)
        }
    }

    pub async fn load_textures(&mut self, fs: &mut dyn FileSystem) -> Result<()> {
        for line in &mut self.lines {
            if let JudgeLineKind::Texture(tex, path) = &mut line.kind {
                *tex = image::load_from_memory(&fs.load_file(path).await.with_context(|| format!("failed to load illustration {path}"))?)?.into();
            }
        }
        Ok(())
    }

    pub fn reset(&mut self) {
        self.lines
            .iter_mut()
            .flat_map(|it| it.notes.iter_mut())
            .for_each(|note| {
                note.judge = JudgeStatus::NotJudged;
                note.protected = false;
            });
        for line in &mut self.lines {
            line.cache.reset(&mut line.notes);
        }
        for video in &mut self.extra.videos {
            if let Err(err) = video.reset() {
                show_error(err.context(tl!("video-load-failed", "path" => video.video_file.path().to_string_lossy())));
            }
        }
    }

    pub fn update(&mut self, res: &mut Resource) {
        for line in &mut self.lines {
            line.object.set_time(res.time);
        }
        // TODO optimize
        let trs = self.lines.iter().map(|it| it.now_transform(res, &self.lines)).collect::<Vec<_>>();
        let mut guard = self.bpm_list.borrow_mut();
        for (index, (line, tr)) in self.lines.iter_mut().zip(trs).enumerate() {
            line.update(res, tr, &mut guard, index);
        }
        drop(guard);
        for effect in &mut self.extra.effects {
            effect.update(res);
        }
    }

    pub fn render(&self, ui: &mut Ui, res: &mut Resource) {
        res.apply_model_of(&Matrix::identity().append_nonuniform_scaling(&Vector::new(if res.config.flip_x() { -1. } else { 1. }, 1.)), |res| {
            for video in &self.extra.videos {
                video.render(res);
            }
        });
        res.apply_model_of(&Matrix::identity().append_nonuniform_scaling(&Vector::new(if res.config.flip_x() { -1. } else { 1. }, -1.)), |res| {
            let mut guard = self.bpm_list.borrow_mut();
            for id in &self.order {
                self.lines[*id].render(ui, res, &self.lines, &mut guard, &self.settings, *id);
            }
            drop(guard);
            res.note_buffer.borrow_mut().draw_all();
            if res.config.sample_count > 1 {
                unsafe { get_internal_gl() }.flush();
                if let Some(target) = &res.chart_target {
                    target.blit();
                }
            }
        });
    }
}
