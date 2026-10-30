//! The message sets the editor can fill a state's box with: ten lines for each state,
//! written for what the seal does in that state's poses. A box holds its lines split by
//! `|`, and the bubble says one at random each time it speaks up again (`next_roll`).

use crate::status::STATES;

/// The sets, by name, in the order the editor offers them.
pub const SETS: [&str; 2] = ["English", "ไทย"];

pub const PER_STATE: usize = 10;

type Lines = [[&'static str; PER_STATE]; STATES.len()];

// in the order of `STATES`: clapping then thinking, swimming in a hard hat with a shovel,
// waving from an ice floe, showing off in sunglasses, crying, and napping (or playing);
// each line tells what the seal is doing in one of the state's poses, clingy and sweet, the
// seal a little one talking to its Papa (หนู to ป๊า in the Thai), many lines two rows
// (`\n`) so the bubble stays narrow
const ENGLISH: Lines = [
    [
        "Clap clap for Papa!\\nNow lemme think~",
        "Flipper on my chin…\\nwait for me, Papa",
        "Blink blink…\\nthinking hard!",
        "Clapping 'cause\\nyou sent me a task!",
        "Chin on flipper… hmm,\\nwhat to do?",
        "Sitting, thinking,\\ndon't go anywhere!",
        "Clap clap, yay,\\nwork's here!",
        "Sitting very still,\\nthinking just for you",
        "Flipper's getting tired…\\nalmost got it!",
        "Blink blink…\\nidea's coming~",
    ],
    [
        "Hard hat on, working\\nto be Papa's favorite!",
        "Shovel go dig dig~\\npraise me after, ok?",
        "Hard hat on,\\nI got this for you!",
        "Swimming thru your code,\\ndon't worry, Papa",
        "Spin spin~\\nit's moving, Papa!",
        "Swam so far…\\nbut I won't give up!",
        "Digging this hard…\\ndo you love me more?",
        "Swimming laps, so no\\nhugging other seals!",
        "Hat's all crooked,\\nbut almost done~",
        "Dig, swim, spin,\\njust for you~",
    ],
    [
        "Waving from my floe,\\ncan you see me?",
        "Sitting on the ice,\\ncome answer me, Papa",
        "Wave wave~ Papaaa,\\nover here!",
        "Waving for a yes,\\npretty please?",
        "This ice is cold,\\nhurry, Papa!",
        "Flipper's tired of waving…\\ndon't ignore me",
        "Waiting on my floe,\\nyour turn, Papa~",
        "Waving you over,\\nhelp me pick?",
        "Floating here waiting,\\npress yes for me?",
        "I'll keep waving\\ntill you come~",
    ],
    [
        "Shades on~\\nall done, Papa!",
        "Shiny shades,\\nPapa's fave did it!",
        "Showing off my shades,\\nam I good?",
        "Sparkle sparkle~\\npraise me, Papa?",
        "So cool, right?\\nWhose kid? Papa's!",
        "Can't take off the shades,\\ntoo much cool",
        "Shades on to celebrate,\\ncheek kiss, please?",
        "Striking a pose for you,\\nwork's done~",
        "Shades and sparkles,\\ncan I have a fish?",
        "Chest out, shades on,\\nare you proud of me?",
    ],
    [
        "Crying in my flippers…\\ndon't be mad, Papa",
        "Tears everywhere…\\nan error, sniff",
        "Sobbing in my flippers,\\ncomfort me, Papa?",
        "Waaah… my flippers\\nare soaked with tears",
        "Sniffle sniffle…\\nstill love me?",
        "Wiping my tears,\\nlet me try again?",
        "Hug me, please…\\nI can't stop crying",
        "Covering my eyes…\\nit broke",
        "Tears flooding the ice…\\nso sad",
        "Crying my heart out,\\nhelp me, Papa?",
    ],
    [
        "Curled up napping… zZ",
        "Dreaming of Papa… zZ",
        "Belly slide on the ice,\\nwheee, waiting for you~",
        "Driving my toy car\\ntill you're free~",
        "Bouncing my ball alone,\\ncome play, Papa?",
        "Eyes closed, snoozing…\\nwake me if you need me",
        "Sliding back and forth…\\nso lonely",
        "Vroom vroom,\\ndriving to Papa!",
        "Waving at you,\\ncome see me later!",
        "Rolling my ball,\\nmissing you~",
    ],
];

const THAI: Lines = [
    [
        "ปรบมือแปะๆ รับงานป๊า\\nแล้วหนูจะคิดให้นะ",
        "เอาครีบเท้าคางคิดอยู่\\nรอหนูแป๊บนะป๊า",
        "กะพริบตาปริบๆ\\nหนูกำลังคิดอยู่น้า",
        "ปรบมือดีใจ\\nป๊าส่งงานมาให้หนูแล้ว",
        "ครีบเท้าคาง… อืมมม\\nทำไงดีน้า",
        "นั่งคิดตาแป๋วอยู่\\nอย่าเพิ่งไปไหนนะป๊า",
        "แปะๆๆ เย้ งานมาแล้ว!",
        "นั่งนิ่งๆ คิดหนักเลย\\nเพื่อป๊าคนเดียว",
        "เท้าคางจนครีบเมื่อย\\nใกล้คิดออกแล้วน้า",
        "กะพริบตาปริบๆ\\nไอเดียใกล้มาแล้ว",
    ],
    [
        "ใส่หมวกตั้งใจทำงาน\\nหนูจะเป็นลูกรักป๊าให้ได้",
        "ถือพลั่วขุดๆๆ\\nเสร็จแล้วชมหนูด้วยนะ",
        "ใส่หมวกนิรภัยแล้ว\\nหนูลุยงานให้ป๊าเอง",
        "ว่ายฝ่าโค้ดอยู่น้า\\nป๊าไม่ต้องห่วง",
        "หมุนตัวติ้วๆ\\nงานเดินแล้วป๊า!",
        "ว่ายจนครีบล้า\\nแต่หนูไม่ยอมแพ้หรอก",
        "ถือพลั่วขยันขนาดนี้\\nป๊ารักหนูมากขึ้นมั้ย",
        "ว่ายวนอยู่ในงาน\\nอย่าแอบไปกอดตัวอื่นนะ",
        "หมวกเบี้ยวแล้ว\\nแต่ใกล้เสร็จแล้วน้าป๊า",
        "ขุดๆ ว่ายๆ หมุนๆ\\nเพื่อป๊าคนเดียวเลย",
    ],
    [
        "หนูโบกมืออยู่บนน้ำแข็ง\\nป๊าเห็นหนูมั้ย",
        "นั่งรอบนก้อนน้ำแข็ง\\nป๊ามาตอบหนูหน่อย",
        "โบกๆๆ ป๊าาา\\nทางนี้ๆ",
        "โบกมือขออนุญาตป๊าน้า\\nนะๆๆ",
        "น้ำแข็งเย็นก้นแล้ว\\nป๊ารีบมาตอบนะ",
        "โบกจนครีบล้าแล้ว\\nอย่าเมินหนูสิ งือ",
        "หนูนั่งรอบนน้ำแข็ง\\nถึงตาป๊าแล้วน้า",
        "โบกมือเรียก\\nป๊าช่วยตัดสินใจให้หนูที",
        "ลอยเท้งเต้งรออยู่\\nกดให้หนูหน่อยน้า",
        "โบกๆ ไม่หยุดเลย\\nจนกว่าป๊าจะมา",
    ],
    [
        "ใส่แว่นดำเท่ๆ\\nเสร็จแล้วป๊า!",
        "แว่นวิ้งๆ\\nลูกรักป๊าทำได้แล้ว",
        "ใส่แว่นอวดหน่อย\\nหนูเก่งมั้ยป๊า",
        "ประกายวิบวับเลย\\nชมหนูหน่อยสิป๊า",
        "เท่มั้ยล่ะ\\nลูกใครเนี่ย ลูกป๊าไง",
        "ถอดแว่นไม่ได้\\nความเท่มันล้น",
        "ใส่แว่นฉลอง\\nขอหอมแก้มหน่อยได้มั้ย",
        "โพสท่าเท่ๆ ให้ป๊าดู\\nงานเสร็จแล้วน้า",
        "แว่นดำ ประกายวิ้ง\\nขอปลาเป็นรางวัลได้มั้ย",
        "ยืดอกใส่แว่นอวด\\nป๊าภูมิใจในตัวหนูมั้ย",
    ],
    [
        "เอาครีบปิดหน้าร้องไห้\\nป๊าอย่าโกรธหนูนะ",
        "น้ำตาไหลพรากๆ\\nerror แว้ว",
        "ร้องไห้ใส่ครีบอยู่\\nป๊าปลอบหนูหน่อย",
        "แง… ครีบเปียกน้ำตาหมดแล้ว",
        "สะอื้นอยู่\\nยังรักหนูอยู่มั้ย",
        "เช็ดน้ำตาแป๊บ\\nให้หนูลองใหม่นะป๊า",
        "ขอกอดหน่อย\\nหนูร้องไห้ไม่หยุดเลย",
        "ฮือๆ ปิดตาไม่กล้ามอง\\nมันพังไปแล้ว",
        "น้ำตาท่วมน้ำแข็งแล้ว\\nหนูเสียใจจัง",
        "ร้องไห้งอแง\\nป๊าช่วยหนูดูหน่อย",
    ],
    [
        "นอนขดหลับปุ๋ย… zZ",
        "หลับฝันถึงป๊าอยู่… zZ",
        "ไถลพุงบนน้ำแข็ง\\nวี้ดดด รอป๊าอยู่นะ",
        "ขับรถของเล่นวนๆ\\nรอป๊าว่างอยู่น้า",
        "เดาะบอลคนเดียว\\nป๊ามาเล่นด้วยกันมั้ย",
        "นอนหลับตาพริ้ม\\nมีงานปลุกหนูได้น้า",
        "ไถลพุงไปมา\\nเหงาจังเลย",
        "บรื้นๆ ขับรถไปหาป๊า",
        "โบกมือให้ป๊า\\nว่างแล้วมาหาหนูนะ",
        "กลิ้งบอลเล่นรอ\\nคิดถึงป๊าจัง",
    ],
];

/// The lines set `set` (from `SETS`) has for the state at `state` in `STATES`.
pub fn lines(set: usize, state: usize) -> &'static [&'static str; PER_STATE] {
    if set == 0 { &ENGLISH[state] } else { &THAI[state] }
}

/// The words set `set` fills the state's box at `state` with: all its lines, split by `|`.
pub fn fill(set: usize, state: usize) -> String {
    lines(set, state).join(" | ")
}

/// The lines in a box's words: split by `|`, each trimmed, the empty ones left out.
pub fn split(words: &str) -> Vec<&str> {
    words.split('|').map(str::trim).filter(|l| !l.is_empty()).collect()
}

/// Which of `n` lines to say next: at random, never the one just said.
pub fn next_roll(last: usize, n: usize) -> usize {
    if n < 2 {
        return 0;
    }
    let r = crate::seal::random() % (n - 1);
    if r >= last % n { r + 1 } else { r }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_fill_the_boxes() {
        for set in 0..SETS.len() {
            for state in 0..STATES.len() {
                let words = fill(set, state);
                // each line comes back out, and the whole fits a box
                assert_eq!(split(&words), lines(set, state).to_vec());
                assert!(words.chars().count() <= crate::status::MAX_CHARS, "{words}");
                assert!(lines(set, state).iter().all(|l| !l.contains('|')));
            }
        }
    }

    #[test]
    fn a_new_line_each_time() {
        assert_eq!(split(" a |b|| c "), ["a", "b", "c"]);
        assert_eq!(next_roll(0, 1), 0);
        let mut last = 0;
        for _ in 0..100 {
            let next = next_roll(last, PER_STATE);
            assert!(next < PER_STATE && next != last);
            last = next;
        }
    }
}
