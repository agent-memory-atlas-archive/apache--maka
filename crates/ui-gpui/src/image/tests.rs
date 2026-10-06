/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing,
 * software distributed under the License is distributed on an
 * "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
 * KIND, either express or implied.  See the License for the
 * specific language governing permissions and limitations
 * under the License.
 */

use super::*;
use crate::BudgetLimits;
use gpui_kit::TestAppContext;
use std::cell::RefCell;

fn png(width: u32, height: u32) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba([241, 83, 120, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}
fn props(bytes: &[u8], key: &str) -> ImageProps {
    ImageProps {
        source: Some(ImageDescriptor {
            key: key.into(),
            digest: format!("sha256:{:x}", Sha256::digest(bytes)),
            mime_type: "image/png".into(),
            total_bytes: bytes.len() as u64,
        }),
        packet: None,
        alt: "示意图 café 😀".into(),
        fit: ImageFit::Contain,
        height: 160.,
        background: maka_ui::appearance::Color(0xffffffff),
        border: maka_ui::appearance::Color(0x888888ff),
        radius: 8.,
    }
}
fn packet(
    owner: &Entity<Owner>,
    cx: &mut App,
    source: &str,
    sequence: u64,
    offset: u64,
    bytes: &[u8],
) -> ImagePacket {
    ImagePacket {
        owner: owner.read(cx).id.clone(),
        source: source.into(),
        sequence,
        offset,
        data: STANDARD.encode(bytes),
    }
}
struct Root {
    owner: Option<Entity<Owner>>,
    visible: bool,
}
impl Render for Root {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut root = div().size_full();
        if self.visible
            && let Some(owner) = &self.owner
        {
            root = root.child(owner.clone());
        }
        root
    }
}
fn input(budget: NativeBudget, observations: Rc<RefCell<Vec<ImageObservation>>>) -> LiveInput {
    LiveInput::new(
        Rc::new(RefCell::new(Some(Input(Rc::new(
            move |key, value, _, _| {
                assert_eq!(key, "observe");
                observations
                    .borrow_mut()
                    .push(serde_json::from_value(value).unwrap());
                Ok(Default::default())
            },
        ))))),
        budget,
    )
}
#[test]
fn original_budget_permits_are_aggregate_and_release_on_actual_drop() {
    let budget = NativeBudget::new(BudgetLimits {
        encoded_bytes: 10,
        pixel_bytes: 20,
        decode_jobs: 1,
    });
    let first = budget.reserve(Kind::Encoded, 8).unwrap();
    assert_eq!(
        budget.reserve(Kind::Encoded, 3).unwrap_err(),
        ImageError::BudgetExhausted
    );
    let job = budget.reserve(Kind::Decode, 1).unwrap();
    assert_eq!(
        budget.reserve(Kind::Decode, 1).unwrap_err(),
        ImageError::BudgetExhausted
    );
    let pixels = budget.reserve(Kind::Pixels, 20).unwrap();
    assert_eq!(budget.usage(), (8, 20, 1));
    drop(first);
    drop(job);
    assert_eq!(budget.usage(), (0, 20, 0));
    drop(pixels);
    assert_eq!(budget.usage(), (0, 0, 0));
}
#[test]
fn real_decoder_verifies_digest_first_frame_dimensions_and_actual_pixel_budget() {
    let bytes = png(3, 2);
    let source = props(&bytes, "a").source.unwrap();
    let budget = NativeBudget::default();
    let encoded = Encoded {
        _permit: budget.reserve(Kind::Encoded, bytes.len()).unwrap(),
        bytes: bytes.clone(),
    };
    let decoded = decode::decode(
        encoded,
        source.clone(),
        budget.clone(),
        budget.reserve(Kind::Decode, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(decoded.image.frame_count(), 1);
    assert_eq!(
        decoded.image.as_bytes(0).unwrap(),
        [120, 83, 241, 255].repeat(6)
    );
    assert_eq!(budget.usage(), (0, 24, 0));
    drop(decoded);
    assert_eq!(budget.usage(), (0, 0, 0));
    let mut wrong = source;
    wrong.digest = "sha256:".to_owned() + &"0".repeat(64);
    let encoded = Encoded {
        _permit: budget.reserve(Kind::Encoded, bytes.len()).unwrap(),
        bytes: bytes.clone(),
    };
    assert!(matches!(
        decode::decode(
            encoded,
            wrong,
            budget.clone(),
            budget.reserve(Kind::Decode, 1).unwrap()
        ),
        Err(ImageError::DigestMismatch)
    ));
    assert_eq!(budget.usage(), (0, 0, 0));
    let small = NativeBudget::new(BudgetLimits {
        encoded_bytes: bytes.len(),
        pixel_bytes: 1,
        decode_jobs: 1,
    });
    let encoded = Encoded {
        _permit: small.reserve(Kind::Encoded, bytes.len()).unwrap(),
        bytes: bytes.clone(),
    };
    assert!(matches!(
        decode::decode(
            encoded,
            props(&bytes, "small").source.unwrap(),
            small.clone(),
            small.reserve(Kind::Decode, 1).unwrap()
        ),
        Err(ImageError::BudgetExhausted)
    ));
    assert_eq!(small.usage(), (0, 0, 0));
    let mut gif = Vec::new();
    {
        let mut encoder = image::codecs::gif::GifEncoder::new(&mut gif);
        for color in [[241, 83, 120, 255], [0, 0, 0, 255]] {
            encoder
                .encode_frame(image::Frame::new(image::RgbaImage::from_pixel(
                    1,
                    1,
                    image::Rgba(color),
                )))
                .unwrap();
        }
    }
    let mut source = props(&gif, "gif").source.unwrap();
    source.mime_type = "image/gif".into();
    let encoded = Encoded {
        _permit: budget.reserve(Kind::Encoded, gif.len()).unwrap(),
        bytes: gif,
    };
    let decoded = decode::decode(
        encoded,
        source,
        budget.clone(),
        budget.reserve(Kind::Decode, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(decoded.image.frame_count(), 1);
    assert_eq!(decoded.image.as_bytes(0).unwrap(), [120, 83, 241, 255]);
    drop(decoded);
    assert_eq!(budget.usage(), (0, 0, 0));
}
#[test]
fn tiny_real_png_huge_header_is_rejected_before_decoding_pixels() {
    let mut bytes = png(1, 1);
    bytes[16..20].copy_from_slice(&8192u32.to_be_bytes());
    bytes[20..24].copy_from_slice(&8192u32.to_be_bytes());
    let mut crc = 0xffffffffu32;
    for b in &bytes[12..29] {
        crc ^= u32::from(*b);
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb88320 } else { 0 };
        }
    }
    bytes[29..33].copy_from_slice(&(!crc).to_be_bytes());
    let budget = NativeBudget::default();
    let source = props(&bytes, "oversize").source.unwrap();
    let encoded = Encoded {
        _permit: budget.reserve(Kind::Encoded, bytes.len()).unwrap(),
        bytes,
    };
    assert!(matches!(
        decode::decode(
            encoded,
            source,
            budget.clone(),
            budget.reserve(Kind::Decode, 1).unwrap()
        ),
        Err(ImageError::LimitExceeded)
    ));
    assert_eq!(budget.usage(), (0, 0, 0));
}
#[test]
fn pure_packets_retain_exact_request_until_original_receipt() {
    let bytes = png(1, 1);
    let mut p = props(&bytes, "a");
    p.packet = Some(ImagePacket {
        owner: "original".into(),
        source: "a".into(),
        sequence: 1,
        offset: 0,
        data: STANDARD.encode(&bytes),
    });
    let values = maka_ui::abi::ValueBindings::default();
    let value = serde_json::to_value(&p).unwrap();
    let first = ImageComponent
        .call(ComponentCall::Mount { props: &value }, &values)
        .unwrap();
    p.packet = None;
    let omitted = serde_json::to_value(&p).unwrap();
    let pending = ImageComponent
        .call(
            ComponentCall::Update {
                state: &first.frame.state,
                props: &omitted,
            },
            &values,
        )
        .unwrap();
    assert_eq!(pending.frame.state["request"], value["packet"]);
    assert_eq!(pending.frame.state["pending"], true);
    let mut changed = value.clone();
    changed["packet"]["offset"] = json!(1);
    assert!(
        ImageComponent
            .call(
                ComponentCall::Update {
                    state: &pending.frame.state,
                    props: &changed
                },
                &values
            )
            .is_err()
    );
    let observation = json!({"owner":"original","revision":1,"source":"a","received":bytes.len(),"phase":"decoding","width":null,"height":null,"error":null,"receipt":{"owner":"other","source":"a","sequence":1,"accepted":true,"error":null}});
    let stale = ImageComponent
        .call(
            ComponentCall::Event {
                state: &pending.frame.state,
                props: &omitted,
                key: "observe",
                input: &observation,
                origin: maka_ui::EventOrigin::Raw,
            },
            &values,
        )
        .unwrap();
    assert_eq!(stale.frame.state["pending"], true);
    let mut matching = observation;
    matching["receipt"]["owner"] = json!("original");
    let settled = ImageComponent
        .call(
            ComponentCall::Event {
                state: &stale.frame.state,
                props: &omitted,
                key: "observe",
                input: &matching,
                origin: maka_ui::EventOrigin::Raw,
            },
            &values,
        )
        .unwrap();
    assert_eq!(settled.frame.state["pending"], false);
    assert_eq!(settled.events[0].name, "observed");
}

#[gpui_kit::test]
fn actual_native_packets_keep_one_owner_and_pixels_through_hidden_and_style_updates(
    cx: &mut TestAppContext,
) {
    let bytes = png(8, 4);
    let initial = props(&bytes, "a");
    let budget = NativeBudget::default();
    let observations = Rc::new(RefCell::new(Vec::new()));
    let live = input(budget.clone(), observations.clone());
    let (root, cx) = cx.add_window_view(|_, cx| Root {
        owner: Some(Owner::new(&initial, live, cx)),
        visible: true,
    });
    let owner = root.read_with(cx, |root, _| root.owner.as_ref().unwrap().clone());
    cx.update(|window, cx| Recipe(initial.clone()).commit(&mut owner.clone(), window, cx));
    cx.run_until_parked();
    let id = owner.read_with(cx, |owner, _| owner.id.clone());
    let half = bytes.len() / 2;
    let mut p = initial.clone();
    cx.update(|window, cx| {
        p.packet = Some(packet(&owner, cx, "a", 1, 0, &bytes[..half]));
        Recipe(p.clone()).commit(&mut owner.clone(), window, cx);
    });
    cx.run_until_parked();
    assert_eq!(
        owner.read_with(cx, |owner, _| owner.observation.received),
        half as u64
    );
    cx.update(|window, cx| Recipe(p.clone()).commit(&mut owner.clone(), window, cx));
    cx.run_until_parked();
    assert_eq!(budget.usage(), (bytes.len(), 0, 0));
    let mut bad = p.clone();
    bad.packet.as_mut().unwrap().owner = "retired".into();
    bad.packet.as_mut().unwrap().sequence = 100;
    cx.update(|window, cx| Recipe(bad).commit(&mut owner.clone(), window, cx));
    cx.run_until_parked();
    assert_eq!(
        owner.read_with(cx, |owner, _| owner.observation.received),
        half as u64
    );
    cx.update(|window, cx| {
        root.update(cx, |root, cx| {
            root.visible = false;
            cx.notify();
        });
        p.packet = Some(packet(&owner, cx, "a", 2, half as u64, &bytes[half..]));
        Recipe(p.clone()).commit(&mut owner.clone(), window, cx);
    });
    cx.run_until_parked();
    assert_eq!(
        owner.read_with(cx, |owner, _| owner.observation.phase),
        ImagePhase::Ready
    );
    assert!(
        observations
            .borrow()
            .iter()
            .any(|o| o.phase == ImagePhase::Ready)
    );
    assert_eq!(budget.usage(), (0, 8 * 4 * 4, 0));
    let old_pixels = owner.read_with(cx, |owner, _| owner.pixels.as_ref().unwrap().clone());
    let pixels = old_pixels.image.id;
    p.packet = None;
    p.height = 220.;
    p.alt = "Replacement alt 中文".into();
    p.fit = ImageFit::Cover;
    cx.update(|window, cx| {
        Recipe(p.clone()).commit(&mut owner.clone(), window, cx);
        root.update(cx, |root, cx| {
            root.visible = true;
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    assert_eq!(owner.read_with(cx, |owner, _| owner.id.clone()), id);
    assert_eq!(
        owner.read_with(cx, |owner, _| owner.pixels.as_ref().unwrap().image.id),
        pixels
    );
    assert_eq!(
        owner.read_with(cx, |owner, _| owner.props.alt.clone()),
        p.alt
    );
    cx.update(|window, cx| {
        assert!(window.has_image_atlas_entry(&owner.read(cx).pixels.as_ref().unwrap().image));
    });
    let handle = cx.update(|window, _| window.window_handle());
    cx.cx.activate_accessibility(handle);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(
        cx.cx
            .accessibility_update(handle)
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.role() == Role::Image && n.label() == Some("Replacement alt 中文"))
    );
    let mut changed = p.clone();
    changed.source.as_mut().unwrap().key = "b".into();
    cx.update(|window, cx| Recipe(changed).commit(&mut owner.clone(), window, cx));
    cx.run_until_parked();
    assert_eq!(budget.usage(), (bytes.len(), 8 * 4 * 4, 0));
    cx.update(|window, _| assert!(!window.has_image_atlas_entry(&old_pixels.image)));
    drop(old_pixels);
    cx.run_until_parked();
    assert_eq!(budget.usage(), (bytes.len(), 0, 0));
}

#[gpui_kit::test]
fn admitted_decoder_survives_retirement_with_original_lease_and_budget(cx: &mut TestAppContext) {
    let bytes = png(2, 2);
    let p = props(&bytes, "a");
    let budget = NativeBudget::default();
    let original = Rc::new(());
    let witness = Rc::downgrade(&original);
    let keep = original.clone();
    drop(original);
    let live = LiveInput::new(
        Rc::new(RefCell::new(Some(Input(Rc::new(move |_, _, _, _| {
            let _ = &keep;
            Ok(Default::default())
        }))))),
        budget.clone(),
    );
    let (root, cx) = cx.add_window_view(|_, cx| Root {
        owner: Some(Owner::new(&p, live.clone(), cx)),
        visible: true,
    });
    let owner = root.read_with(cx, |r, _| r.owner.as_ref().unwrap().clone());
    let weak = owner.downgrade();
    let (release, gate) = tokio::sync::oneshot::channel();
    cx.update(|window, cx| {
        owner.update(cx, |owner, _| owner.decode_gate = Some(gate));
        let mut deliver = p.clone();
        deliver.packet = Some(packet(&owner, cx, "a", 1, 0, &bytes));
        Recipe(deliver).commit(&mut owner.clone(), window, cx);
    });
    cx.run_until_parked();
    assert_eq!(budget.usage(), (bytes.len(), 0, 1));
    live.0.borrow_mut().take();
    cx.update(|window, cx| {
        Recipe(p.clone()).deactivate(&mut owner.clone(), window, cx);
        root.update(cx, |r, cx| {
            r.owner = None;
            cx.notify();
        });
    });
    drop(owner);
    cx.run_until_parked();
    assert!(weak.upgrade().is_none());
    assert!(witness.upgrade().is_some());
    assert_eq!(budget.usage(), (bytes.len(), 0, 1));
    release.send(()).unwrap();
    cx.run_until_parked();
    assert_eq!(budget.usage(), (0, 0, 0));
    assert!(witness.upgrade().is_none());
}
#[gpui_kit::test]
fn a_cancelled_source_cannot_resurrect_after_descriptor_aba(cx: &mut TestAppContext) {
    let bytes = png(2, 2);
    let p = props(&bytes, "a");
    let budget = NativeBudget::default();
    let obs = Rc::new(RefCell::new(Vec::new()));
    let (root, cx) = cx.add_window_view(|_, cx| Root {
        owner: Some(Owner::new(&p, input(budget.clone(), obs), cx)),
        visible: true,
    });
    let owner = root.read_with(cx, |r, _| r.owner.as_ref().unwrap().clone());
    let (release, gate) = tokio::sync::oneshot::channel();
    cx.update(|window, cx| {
        owner.update(cx, |owner, _| owner.decode_gate = Some(gate));
        let mut deliver = p.clone();
        deliver.packet = Some(packet(&owner, cx, "a", 1, 0, &bytes));
        Recipe(deliver).commit(&mut owner.clone(), window, cx);
    });
    cx.run_until_parked();
    let mut empty = p.clone();
    empty.source = None;
    cx.update(|window, cx| {
        Recipe(empty).commit(&mut owner.clone(), window, cx);
        Recipe(p.clone()).commit(&mut owner.clone(), window, cx);
    });
    cx.run_until_parked();
    assert_eq!(budget.usage(), (bytes.len() * 2, 0, 1));
    release.send(()).unwrap();
    cx.run_until_parked();
    assert_eq!(
        owner.read_with(cx, |o, _| o.observation.phase),
        ImagePhase::Receiving
    );
    assert_eq!(owner.read_with(cx, |o, _| o.observation.received), 0);
    assert!(owner.read_with(cx, |o, _| o.pixels.is_none()));
    assert_eq!(budget.usage(), (bytes.len(), 0, 0));
}

#[gpui_kit::test]
fn rejected_publication_then_source_aba_settles_original_packet(cx: &mut TestAppContext) {
    let bytes = png(2, 2);
    let initial = props(&bytes, "a");
    let observations = Rc::new(RefCell::new(Vec::<ImageObservation>::new()));
    let accepting = Rc::new(RefCell::new(false));
    let sink = observations.clone();
    let admitted = accepting.clone();
    let live = LiveInput::new(
        Rc::new(RefCell::new(Some(Input(Rc::new(move |_, value, _, _| {
            if !*admitted.borrow() {
                return Err(maka_ui::Error::Invalid(
                    "parent transaction rejected".into(),
                ));
            }
            sink.borrow_mut()
                .push(serde_json::from_value(value).unwrap());
            Ok(Default::default())
        }))))),
        NativeBudget::default(),
    );
    let (root, cx) = cx.add_window_view(|_, cx| Root {
        owner: Some(Owner::new(&initial, live, cx)),
        visible: true,
    });
    let owner = root.read_with(cx, |r, _| r.owner.as_ref().unwrap().clone());
    let mut deliver = initial.clone();
    cx.update(|window, cx| {
        deliver.packet = Some(packet(&owner, cx, "a", 1, 0, &bytes[..bytes.len() / 2]));
        Recipe(deliver.clone()).commit(&mut owner.clone(), window, cx);
    });
    cx.run_until_parked();
    assert!(observations.borrow().is_empty());
    assert!(owner.read_with(cx, |o, _| o.needs_publication));
    let values = maka_ui::abi::ValueBindings::default();
    let delivered = serde_json::to_value(&deliver).unwrap();
    let pending = ImageComponent
        .call(ComponentCall::Mount { props: &delivered }, &values)
        .unwrap();
    assert_eq!(pending.frame.state["pending"], true);
    let mut empty = initial.clone();
    empty.source = None;
    cx.update(|window, cx| {
        Recipe(empty).commit(&mut owner.clone(), window, cx);
        Recipe(initial.clone()).commit(&mut owner.clone(), window, cx);
    });
    cx.run_until_parked();
    *accepting.borrow_mut() = true;
    cx.update(|window, cx| Recipe(deliver.clone()).commit(&mut owner.clone(), window, cx));
    cx.run_until_parked();
    assert_eq!(owner.read_with(cx, |o, _| o.observation.received), 0);
    let observation = observations.borrow().last().unwrap().clone();
    let receipt = observation.receipt.as_ref().unwrap();
    assert!(!receipt.accepted);
    assert_eq!(receipt.error, Some(ImageError::PacketRejected));
    assert_eq!(receipt.sequence, 1);
    let settled = ImageComponent
        .call(
            ComponentCall::Event {
                state: &pending.frame.state,
                props: &serde_json::to_value(&initial).unwrap(),
                key: "observe",
                input: &serde_json::to_value(observation).unwrap(),
                origin: maka_ui::EventOrigin::Raw,
            },
            &values,
        )
        .unwrap();
    assert_eq!(settled.frame.state["pending"], false);
    cx.update(|window, cx| {
        deliver.packet = Some(packet(&owner, cx, "a", 2, 0, &bytes));
        Recipe(deliver).commit(&mut owner.clone(), window, cx);
    });
    cx.run_until_parked();
    assert_eq!(
        owner.read_with(cx, |o, _| o.observation.phase),
        ImagePhase::Ready
    );
}

#[test]
fn real_webp_decodes_but_nested_oversize_vp8_is_rejected() {
    let image = image::RgbaImage::from_pixel(3, 2, image::Rgba([241, 83, 120, 255]));
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut encoded, image::ImageFormat::WebP)
        .unwrap();
    let budget = NativeBudget::default();
    let run = |bytes: Vec<u8>| {
        let mut source = props(&bytes, "webp").source.unwrap();
        source.mime_type = "image/webp".into();
        decode::decode(
            Encoded {
                _permit: budget.reserve(Kind::Encoded, bytes.len()).unwrap(),
                bytes,
            },
            source,
            budget.clone(),
            budget.reserve(Kind::Decode, 1).unwrap(),
        )
    };
    let decoded = run(encoded.into_inner()).unwrap();
    assert_eq!(
        decoded.image.as_bytes(0).unwrap(),
        [120, 83, 241, 255].repeat(6)
    );
    drop(decoded);
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(b"WEBPVP8X");
    bytes.extend_from_slice(&10u32.to_le_bytes());
    bytes.extend_from_slice(&[0; 10]); // canvas 1x1
    bytes.extend_from_slice(b"VP8 ");
    bytes.extend_from_slice(&10u32.to_le_bytes());
    bytes.extend_from_slice(&[0x10, 0, 0, 0x9d, 0x01, 0x2a, 0xff, 0x3f, 0xff, 0x3f]);
    assert!(matches!(run(bytes), Err(ImageError::InvalidImage)));
    assert_eq!(budget.usage(), (0, 0, 0));
}
