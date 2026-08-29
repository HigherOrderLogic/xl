// Copyright 2026 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::mem;

use accesskit::Role;
use masonry::core::{
    AccessCtx, ActionCtx, ChildrenIds, CollectionWidget, ErasedAction, LayoutCtx, MeasureCtx,
    PaintCtx, PropertiesMut, PropertiesRef, RegisterCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Rect, Size};
use masonry::layout::{AsUnit, LayoutSize, LenReq, Length, SizeDef};
use masonry::peniko::Color;
use masonry::properties::{TrackColor, types::CrossAxisAlignment};
use masonry::widgets::{Flex, Slider, SliderMoved};

use crate::demo::CONTENT_GAP;

#[derive(PartialEq, Debug, Copy, Clone)]
enum Component {
    R,
    G,
    B,
    A,
}

impl Component {
    const ALL: [Self; 4] = [Self::R, Self::G, Self::B, Self::A];

    fn update(self, color: &mut Color, value: f32) {
        color.components[match self {
            Self::R => 0,
            Self::G => 1,
            Self::B => 2,
            Self::A => 3,
        }] = value;
    }

    fn get(self, color: &Color) -> f32 {
        color.components[match self {
            Self::R => 0,
            Self::G => 1,
            Self::B => 2,
            Self::A => 3,
        }]
    }

    fn visual_color(self) -> Color {
        match self {
            Self::R => Color::from_rgb8(0xff, 0x00, 0x00),
            Self::G => Color::from_rgb8(0x00, 0xff, 0x00),
            Self::B => Color::from_rgb8(0x00, 0x00, 0xff),
            Self::A => Color::WHITE,
        }
    }
}

#[derive(PartialEq, Debug)]
pub(crate) struct ColorSelected {
    pub color: Color,
}

#[derive(PartialEq, Debug)]
struct ColorSliderMoved {
    component: Component,
    value: f64,
}

struct ColorSlider {
    component: Component,
    slider: WidgetPod<Slider>,
}

impl ColorSlider {
    fn new(component: Component, value: f64) -> Self {
        Self {
            component,
            slider: Slider::new(0., 1., value)
                .prepare()
                .with_props(TrackColor {
                    active: component.visual_color(),
                    ..Default::default()
                })
                .to_pod(),
        }
    }

    fn set_value(this: &mut WidgetMut<'_, Self>, value: f64) {
        let mut slider = this.ctx.get_mut(&mut this.widget.slider);
        Slider::set_value(&mut slider, value);
    }
}

impl Widget for ColorSlider {
    type Action = ColorSliderMoved;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.slider);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        _len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        ctx.redirect_measurement(&mut self.slider, axis, cross_length)
    }

    fn on_action(
        &mut self,
        ctx: &mut ActionCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        action: &ErasedAction,
        _source: WidgetId,
    ) {
        if let Some(SliderMoved { value }) = action.downcast_ref::<SliderMoved>() {
            ctx.submit_action::<Self::Action>(ColorSliderMoved {
                component: self.component,
                value: *value,
            });
            ctx.set_handled();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let size = ctx.compute_size(&mut self.slider, SizeDef::fit(size), size.into());
        ctx.run_layout(&mut self.slider, size);
        ctx.place_child(&mut self.slider, Point::ORIGIN);
        ctx.derive_baselines(&self.slider);
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut accesskit::Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.slider.id()])
    }
}

pub(crate) struct HorizontalColorPicker {
    color: Color,
    widget: WidgetPod<Flex>,
}

impl HorizontalColorPicker {
    pub(crate) fn new(color: Color) -> Self {
        let mut body = Flex::row().cross_axis_alignment(CrossAxisAlignment::Stretch);

        for c in Component::ALL {
            let widget = ColorSlider::new(c, c.get(&color) as f64).prepare();
            body = mem::replace(&mut body, Flex::row())
                .with(widget, 1.)
                .with_fixed_spacer((CONTENT_GAP.get() / 2.0).px());
        }

        body = body.with_fixed_spacer(20.0.px());

        Self {
            color,
            widget: body.prepare().to_pod(),
        }
    }

    #[expect(dead_code, reason = "Currently unused, available for future use.")]
    pub(crate) fn set_color(this: &mut WidgetMut<'_, Self>, color: Color) {
        {
            let mut body = this.ctx.get_mut(&mut this.widget.widget);
            for (index, component) in Component::ALL.into_iter().enumerate() {
                let value = component.get(&color);
                let mut slider = Flex::get_mut(&mut body, index * 2);
                let mut slider = slider.downcast::<ColorSlider>();
                ColorSlider::set_value(&mut slider, value as f64);
            }
        }
        this.widget.color = color;
        this.ctx.request_paint_only();
    }
}

impl Widget for HorizontalColorPicker {
    type Action = ColorSelected;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.widget);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        let auto_length = len_req.into();
        let context_size = LayoutSize::maybe(axis.cross(), cross_length);
        ctx.compute_length(
            &mut self.widget,
            auto_length,
            context_size,
            axis,
            cross_length,
        )
    }

    fn on_action(
        &mut self,
        ctx: &mut ActionCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        action: &ErasedAction,
        _source: WidgetId,
    ) {
        if let Some(ColorSliderMoved { component, value }) =
            action.downcast_ref::<ColorSliderMoved>()
        {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "Value is in range of [0.0, 1.0]"
            )]
            component.update(&mut self.color, *value as f32);
            ctx.submit_action::<Self::Action>(ColorSelected { color: self.color });
            ctx.request_paint_only();
            ctx.set_handled();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let content_size = ctx.compute_size(&mut self.widget, SizeDef::fit(size), size.into());
        ctx.run_layout(&mut self.widget, content_size);
        ctx.place_child(&mut self.widget, Point::ORIGIN);
        ctx.derive_baselines(&self.widget);
    }

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let content_box = ctx.content_box();
        painter.fill_rect(
            Rect::new(
                content_box.x1 - 20.0,
                content_box.y0,
                content_box.x1,
                content_box.y1,
            ),
            self.color,
        );
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut accesskit::Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.widget.id()])
    }
}
