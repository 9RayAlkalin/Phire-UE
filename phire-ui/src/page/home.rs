phire::tl_file!("home");

use std::{sync::Arc};

use super::{LibraryPage, NextPage, Page, ResPackPage, SFader, SettingsPage, SharedState};
use crate::{
    client::{recv_raw, Client, LoginParams, User, UserManager},
    dir, get_data, get_data_mut,
    icons::Icons,
    login::Login,
    save_data,
    scene::ProfileScene,
    sync_data,
};
use ::rand::{random, thread_rng, Rng};
use anyhow::Result;
use image::DynamicImage;
use macroquad::prelude::*;
use phire::{
    ext::{semi_black, semi_white, RectExt, SafeTexture, ScaleType},
    info::ChartInfo,
    scene::{show_error, NextScene},
    task::Task,
    ui::{button_hit_large, rounded_rect, DRectButton, Ui},
};
use serde::Deserialize;
use tracing::warn;

const BOARD_SWITCH_TIME: f32 = 4.;
const BOARD_TRANSIT_TIME: f32 = 1.2;

pub struct HomePage {
    character: SafeTexture,
    icons: Arc<Icons>,

    btn_play: DRectButton,
    // btn_event: DRectButton,
    btn_respack: DRectButton,
    // btn_msg: DRectButton,
    btn_settings: DRectButton,
    btn_user: DRectButton,

    next_page: Option<NextPage>,

    login: Login,
    update_task: Option<Task<Result<User>>>,

    need_back: bool,
    sf: SFader,

    board_task: Option<Task<Result<Option<DynamicImage>>>>,
    board_last_time: f32,
    board_last: Option<String>,
    board_tex_last: Option<SafeTexture>,
    board_tex: Option<SafeTexture>,
    board_dir: bool,

    has_new_task: Option<Task<Result<bool>>>,
    has_new: bool,
}

impl HomePage {
    pub async fn new() -> Result<Self> {
        let character = SafeTexture::from(load_texture("char.png").await?).with_mipmap();
        let update_task = if get_data().config.offline_mode {
            None
        } else if let Some(u) = &get_data().me {
            UserManager::request(u.id);
            Some(Task::new(async {
                Client::login(LoginParams::RefreshToken {
                    token: &get_data().tokens.as_ref().unwrap().1,
                })
                .await?;
                Client::get_me().await
            }))
        } else {
            None
        };
        Ok(Self {
            character,
            icons: Arc::new(Icons::new().await?),

            btn_play: DRectButton::new().with_radius(0.00).with_delta(-0.006).with_elevation(0.000).no_sound(),
            // btn_event: DRectButton::new().with_elevation(0.002).no_sound(),
            btn_respack: DRectButton::new().with_radius(0.00).with_delta(-0.003).with_elevation(0.000).no_sound(),
            // btn_msg: DRectButton::new().with_radius(0.03).with_delta(-0.003).with_elevation(0.002),
            btn_settings: DRectButton::new().with_radius(0.00).with_delta(-0.003).with_elevation(0.000),
            btn_user: DRectButton::new().with_delta(-0.000),

            next_page: None,

            login: Login::new(),
            update_task,

            need_back: false,
            sf: SFader::new(),

            board_task: None,
            board_last_time: f32::NEG_INFINITY,
            board_last: None,
            board_tex_last: None,
            board_tex: None,
            board_dir: false,

            has_new_task: None,
            has_new: false,
        })
    }
}

impl HomePage {
    fn fetch_has_new(&mut self) {
        let time = get_data().message_check_time.unwrap_or_default();
        self.has_new_task = Some(Task::new(async move {
            #[derive(Deserialize)]
            struct Resp {
                has: bool,
            }
            let resp: Resp = recv_raw(Client::get("/message/has_new").query(&[("checked", time)]))
                .await?
                .json()
                .await?;
            Ok(resp.has)
        }));
    }
}

impl Page for HomePage {
    fn label(&self) -> std::borrow::Cow<'static, str> {
        "PHIRE".into()
    }

    fn enter(&mut self, s: &mut SharedState) -> Result<()> {
        if self.need_back {
            self.sf.enter(s.t);
            self.need_back = false;
        }
        self.fetch_has_new();
        Ok(())
    }

    fn touch(&mut self, touch: &Touch, s: &mut SharedState) -> Result<bool> {
        if self.sf.transiting() {
            return Ok(true);
        }
        let t = s.t;
        if self.login.touch(touch, s.t) {
            return Ok(true);
        }
        if self.btn_play.touch(touch, t) {
            button_hit_large();
            self.next_page = Some(NextPage::Overlay(Box::new(LibraryPage::new(Arc::clone(&self.icons), s.icons.clone())?)));
            return Ok(true);
        }
        // if self.btn_event.touch(touch, t) {
        //     button_hit_large();
        //     if get_data().me.is_none() {
        //         self.login.enter(t);
        //     } else {
        //         self.next_page = Some(NextPage::Overlay(Box::new(EventPage::new(Arc::clone(&self.icons), s.icons.clone()))));
        //     }
        //     return Ok(true);
        // }
        if self.btn_respack.touch(touch, t) {
            button_hit_large();
            self.next_page = Some(NextPage::Overlay(Box::new(ResPackPage::new(Arc::clone(&self.icons))?)));
            return Ok(true);
        }
        // if self.btn_msg.touch(touch, t) {
        //     self.next_page = Some(NextPage::Overlay(Box::new(MessagePage::new())));
        //     return Ok(true);
        // }
        if self.btn_settings.touch(touch, t) {
            self.next_page = Some(NextPage::Overlay(Box::new(SettingsPage::new(self.icons.lang.clone()))));
            return Ok(true);
        }
        if self.btn_user.touch(touch, t) {
            if let Some(me) = &get_data().me {
                self.need_back = true;
                self.sf.goto(t, ProfileScene::new(me.id, self.icons.user.clone(), s.icons.clone()));
            } else {
                self.login.enter(t);
            }
            return Ok(true);
        }
        Ok(false)
    }

    fn update(&mut self, s: &mut SharedState) -> Result<()> {
        let t = s.t;
        self.login.update(t)?;
        if let Some(task) = &mut self.update_task {
            if let Some(res) = task.take() {
                match res {
                    Err(err) => {
                        // wtf bro
                        if format!("{err:?}").contains("invalid token") {
                            get_data_mut().me = None;
                            get_data_mut().tokens = None;
                            let _ = save_data();
                            sync_data();
                        }
                        show_error(err.context(tl!("failed-to-update")));
                    }
                    Ok(val) => {
                        get_data_mut().me = Some(val);
                        save_data()?;
                    }
                }
                self.update_task = None;
            }
        }
        if self.board_task.is_none() && t - self.board_last_time > BOARD_SWITCH_TIME {
            let charts = &get_data().charts;
            let last_index = self
                .board_last
                .as_ref()
                .and_then(|path| charts.iter().position(|it| &it.local_path == path));
            if charts.is_empty() || (charts.len() == 1 && last_index.is_some()) {
                self.board_task = Some(Task::new(async move { Ok(None) }));
            } else {
                let mut index = thread_rng().gen_range(0..(charts.len() - last_index.is_some() as usize));
                if last_index.map_or(false, |it| it <= index) {
                    index += 1;
                }
                let path = charts[index].local_path.clone();
                let dir = phire::dir::Dir::new(format!("{}/{}", dir::charts()?, path))?;
                self.board_last = Some(path);
                self.board_task = Some(Task::new(async move {
                    let info: ChartInfo = serde_yaml::from_reader(dir.open("info.yml")?)?;
                    let bytes = dir.read(info.illustration)?;
                    Ok(Some(image::load_from_memory(&bytes)?))
                }));
            }
        }
        if let Some(task) = &mut self.board_task {
            if let Some(res) = task.take() {
                match res {
                    Err(err) => {
                        warn!("failed to load illustration for board: {:?}", err);
                    }
                    Ok(image) => {
                        if let Some(image) = image {
                            let tex: SafeTexture = image.into();
                            self.board_tex_last = std::mem::replace(&mut self.board_tex, Some(tex));
                            self.board_dir = random();
                        }
                    }
                }
                self.board_last_time = t;
                self.board_task = None;
            }
        }
        if let Some(task) = &mut self.has_new_task {
            if let Some(res) = task.take() {
                match res {
                    Err(err) => {
                        warn!("fail to load has new {:?}", err);
                    }
                    Ok(has) => {
                        self.has_new = has;
                    }
                }
            }
        }
        Ok(())
    }

    fn render(&mut self, ui: &mut Ui, s: &mut SharedState) -> Result<()> {
        let t = s.t;
        let pad = 0.04;

        let offset = s.gyro_offset;

        s.render_fader(ui, |ui, c| {
            let r = Rect::new(offset.x * 0.4 - 0.9, offset.y * 0.4 - ui.top + 0.1, 1.5, 1.5);
            ui.fill_rect(r, (*self.character, r, ScaleType::CropCenter, c));
        });

        // play button
        // let top = s.render_fader(ui, |ui, c| {
        //     let r = Rect::new(0.60, -0.30, 0.3, 0.2);
        //     let top = r.bottom();
        //     let (r, path) = self.btn_play.render_shadow(ui, r, t, c.a, |_| semi_black(0.4 * c.a));
        //     if let Some(cur) = &self.board_tex {
        //         let p = (t - self.board_last_time) / BOARD_TRANSIT_TIME;
        //         if p > 1. {
        //             self.board_tex_last = None;
        //             ui.fill_path(&path, (**cur, r, ScaleType::CropCenter, c));
        //         } else if let Some(last) = &self.board_tex_last {
        //             let (cur, last) = if self.board_dir { (last, cur) } else { (cur, last) };
        //             let p = 1. - (1. - p).powi(3);
        //             let p = if self.board_dir { 1. - p } else { p };
        //             let rad = self.btn_play.config.radius;
        //             rounded_rect(ui, r, rad, |ui| {
        //                 let mut nr = r;
        //                 nr.h = r.h * (1. - p);
        //                 ui.fill_rect(nr, (**last, nr, ScaleType::CropCenter, c));

        //                 nr.h = r.h * p;
        //                 nr.y = r.bottom() - nr.h;
        //                 ui.fill_rect(nr, (**cur, nr, ScaleType::CropCenter, c));
        //             });
        //         } else {
        //             ui.fill_path(&path, (**cur, r, ScaleType::CropCenter, semi_white(p * c.a)));
        //         }
        //     }
        //     ui.fill_path(&path, (semi_black(0.7 * c.a), (r.x, r.y), Color::default(), (r.x + 0.6, r.y)));
        //     ui.text(tl!("play")).pos(r.x + pad, r.y + pad).color(c).size(0.6 * r.w / r.h).draw();
        //     let r = Rect::new(r.x + 0.02, r.bottom() - 0.10, 0.08, 0.08);
        //     ui.fill_rect(r, (*self.icons.play, r, ScaleType::Fit, semi_white(0.6 * c.a)));
        //     top
        // });

        let text_and_icon = |ui: &mut Ui, r: Rect, btn: &mut DRectButton, text, icon, c: Color| {
            let ow = r.w;
            let (r, _) = btn.render_shadow(ui, r, t, c.a, |_| semi_black(0.4 * c.a));
            let ir = Rect::new(r.x + 0.02, r.bottom() - 0.08, 0.14, 0.14);
            ui.text(text).pos(r.x + 0.026, r.y + 0.026).size(0.6 * r.w / ow).color(c).draw();
            ui.fill_rect(
                {
                    let mut ir = ir;
                    ir.h = ir.h.min(r.bottom() - ir.y);
                    ir
                },
                (icon, ir, ScaleType::Fit, semi_white(0.4 * c.a)),
            );
        };

        let r = s.render_fader(ui, |ui, c| {
            let r = Rect::new(offset.x * 0.6 + 0.70, offset.y * 0.6 - 0.30, 1.3, 0.2);
            text_and_icon(ui, r, &mut self.btn_play, tl!("play"), *self.icons.play, c);
            r
        });

        // let r = s.render_fader(ui, |ui, c| {
        //     let r = Rect::new(0., top, 0.38, 0.23);
        //     text_and_icon(ui, r, &mut self.btn_event, tl!("event"), *self.icons.medal, c);
        //     r
        // });

        let r = s.render_fader(ui, |ui, c| {
            let r = Rect::new(r.left(), r.bottom() + 0.02, 1.3, 0.2);
            text_and_icon(ui, r, &mut self.btn_respack, tl!("respack"), *self.icons.respack, c);
            r
        });

        s.render_fader(ui, |ui, c| {
            // let r = Rect::new(lf, top, 0.11, 0.11);
            // let (r, _) = self.btn_msg.render_shadow(ui, r, t, c.a, |_| semi_black(0.4 * c.a));
            // let r = r.feather(-0.01);
            // ui.fill_rect(r, (*self.icons.msg, r, ScaleType::Fit, c));
            // if self.has_new {
            //     let pad = 0.007;
            //     ui.fill_circle(r.right() - pad, r.y + pad, 0.01, Color { a: c.a, ..RED });
            // }

            let r = Rect::new(r.left(), r.bottom() + 0.02, 1.3, 0.2);
            // let (r, _) = self.btn_settings.render_shadow(ui, r, t, c.a, |_| semi_black(0.4 * c.a));
            // let r = r.feather(0.004);
            // ui.fill_rect(r, (*self.icons.settings, r, ScaleType::Fit, c));
            text_and_icon(ui, r, &mut self.btn_settings, tl!("settings"), *self.icons.settings, c);
        });

        s.fader.roll_back();
        s.render_fader(ui, |ui, c| {
            let rad = 0.05;
            let ct = (0.9, -ui.top + 0.1);
            self.btn_user.config.radius = rad;
            let r = Rect::new(ct.0, ct.1, 0., 0.).feather(rad);
            let (r, _) = self.btn_user.build(ui, t, r);
            ui.avatar(
                ct.0,
                ct.1,
                r.w / 2.,
                c,
                t,
                get_data()
                    .me
                    .as_ref()
                    .map(|user| UserManager::opt_avatar(user.id, &self.icons.user))
                    .unwrap_or(Err(self.icons.user.clone())),
            );
            let rt = ct.0 - rad - 0.02;
            if let Some(me) = &get_data().me {
                ui.text(&me.name).pos(rt, r.center().y + 0.002).anchor(1., 1.).size(0.6).color(c).draw();
                ui.text(format!("{:.2}", me.rks))
                    .pos(rt, r.center().y + 0.008)
                    .anchor(1., 0.)
                    .size(0.4)
                    .color(Color { a: c.a * 0.6, ..c })
                    .draw();
            } else {
                ui.text(tl!("not-logged-in"))
                    .pos(rt, r.center().y)
                    .anchor(1., 0.5)
                    .no_baseline()
                    .size(0.6)
                    .color(c)
                    .draw();
            }
        });
        self.login.render(ui, t);
        self.sf.render(ui, t);
        Ok(())
    }

    fn next_page(&mut self) -> NextPage {
        self.next_page.take().unwrap_or_default()
    }

    fn next_scene(&mut self, s: &mut SharedState) -> NextScene {
        self.sf.next_scene(s.t).unwrap_or_default()
    }
}
