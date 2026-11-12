# Taikyoku Shogi - Piece Movement Reference

Complete movement reference for all 209 piece types and their promoted forms.

## Movement Diagram Legend

```text
  #    piece position
  o    step (1 square)
  2-7  limited range (up to N squares)
  |    slide unlimited (vertical)
  -    slide unlimited (horizontal)
  / \  slide unlimited (diagonal)
  *    jump (leaps over intervening pieces)
  .    cannot move here
```

Diagrams show movement from **Black's perspective** (forward = up).

**Special abilities** (not fully shown in diagrams):

| Symbol | Meaning |
| -------- | --------- |
| **Hook** | Slides along a line, then optionally turns 90 degrees and continues sliding |
| **Area** | Takes multiple steps per turn (like a Lion in Chu Shogi) |
| **Range capture** | Flies over any number of pieces, capturing all of lower rank |
| **Igui** | Captures an adjacent piece without leaving its square |

---

## Base Pieces (209)

### Angry Boar (AB) 嗔猪

```text
. . . . .
. o . o .
. o # o .
. o . o .
. . . . .
```

- Promotes to: **+FB**

---

### Angle Mover (B)

```text
\ . . . /
. \ . / .
. . # . .
. / . \ .
/ . . . \
```

- Promotes to: **Dragon Horse**

---

### Running Bear (BA)

```text
. . . . /
. o o / .
. o # o .
. / o o .
/ . . . .
```

- Promotes to: **Free Bear**

---

### Blind Bear (BB)

```text
. . . . .
. . . . .
. o # o .
. . . . .
. . . . .
```

- Promotes to: **Flying Stag**

---

### Beast Cadet (BC)

```text
2 . 2 . 2
. 1 1 1 .
2 1 # 1 2
. 1 . 1 .
2 . . . 2
```

- Promotes to: **Beast Officer**

---

### Buddhist Devil (BD)

```text
2 . . . 2
. 1 . 1 .
. o # o .
. . o . .
. . . . .
```

- Promotes to: **Heavenly Tetrarch**

---

### Bear Soldier (BE)

```text
\ . | . /
. \ | / .
2 1 # 1 2
. . o . .
. . . . .
```

- Promotes to: **Strong Bear**

---

### Angle General (BG)

```text
. . . . .
. . . . .
. . # . .
. . . . .
. . . . .
```

- Promotes to: **Rain Demon**
- Range capture: NE, SE, SW, NW

---

### Blind Dog (BI)

```text
. . . . .
. o . o .
. o # o .
. . o . .
. . . . .
```

- Promotes to: **Fierce Stag**

---

### Blue Dragon (BL)

```text
. . | . /
. . | / .
2 1 # 1 2
. . | . .
. . | . .
```

- Promotes to: **Divine Dragon**

---

### Blind Monkey (BM)

```text
. . . . .
. . . . .
. o # o .
. . . . .
. . . . .
```

- Promotes to: **Flying Stag**

---

### Cannon Soldier (BN)

```text
2 . 2 . 2
. 1 1 1 .
2 1 # 1 2
. . o . .
. . . . .
```

- Promotes to: **Cannon General**

---

### Beast Officer (BO)

```text
2 . 2 . 2
. 1 1 1 .
2 1 # 1 2
. 1 . 1 .
2 . . . 2
```

- Promotes to: **Beast Bird**

---

### Boar Soldier (BS)

```text
\ . | . /
. \ | / .
2 1 # 1 2
. . o . .
. . . . .
```

- Promotes to: **Running Boar**

---

### Blind Tiger (BT)

```text
. . . . .
. o . o .
. o # o .
. o o o .
. . . . .
```

- Promotes to: **Flying Stag**

---

### Copper General (C)

```text
. . . . .
. o o o .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Side Mover**

---

### Capricorn (CA)

```text
\ . . . /
. \ o / .
. o # o .
. / o \ .
/ . . . \
```

- Promotes to: **Hook Mover**
- Hook move (diag): slides, then optionally turns 90 degrees

---

### Huai Chicken (CC)

```text
. . . . .
. o . o .
. o # o .
. . o . .
. . . . .
```

- Promotes to: **Wizard Stork**

---

### Ceramic Dove (CD)

```text
\ . 2 . /
. \ 1 / .
2 1 # 1 2
. / 1 \ .
/ . 2 . \
```

---

### Cloud Eagle (CE)

```text
2 . . . 2
. 1 o 1 .
. o # o .
. . o . .
. . . . .
```

- Promotes to: **Strong Eagle**

---

### Chicken General (CG)

```text
. . 2 . .
. . 1 . .
. . # . .
. o . o .
. . . . .
```

- Promotes to: **Free Chicken**

---

### Chariot Soldier (CH)

```text
. . | . .
. . | . .
2 1 # 1 2
. . | . .
. . | . .
```

- Promotes to: **Heavenly Tetrarch King**

---

### Stone Chariot (CI)

```text
. . | . .
. o | o .
2 1 # 1 2
. . | . .
. . | . .
```

- Promotes to: **Walking Heron**

---

### Flying Chicken (CK)

```text
. . . . .
. o . o .
. o # o .
. . . . .
. . . . .
```

- Promotes to: **Raiding Hawk**

---

### Cloud Dragon (CL)

```text
\ . . . /
. \ o / .
. o # o .
. / | \ .
/ . | . \
```

- Promotes to: **Great Dragon**

---

### Climbing Monkey (CM)

```text
. . . . .
. o o o .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Fierce Stag**

---

### Center Standard (CN)

```text
2 . | . 2
. 1 | 1 .
- - # - -
. 1 | 1 .
2 . | . 2
```

- Promotes to: **Front Standard**

---

### Fowl Officer (CO)

```text
2 . 2 . 2
. 1 1 1 .
2 1 # 1 2
. 1 . 1 .
2 . . . 2
```

- Promotes to: **Fowl**

---

### Crown Prince (CP)

```text
. . . . .
. o o o .
. o # o .
. o o o .
. . . . .
```

- Promotes to: **King**

---

### Copper Chariot (CR)

```text
2 . | . 2
. 1 | 1 .
. . # . .
. . | . .
. . | . .
```

- Promotes to: **Copper Elephant**

---

### Cat Sword (CS)

```text
. . . . .
. o . o .
. . # . .
. o . o .
. . . . .
```

- Promotes to: **Dragon Horse**

---

### Fowl Cadet (CT)

```text
2 . 2 . 2
. 1 1 1 .
2 1 # 1 2
. 1 . 1 .
2 . . . 2
```

- Promotes to: **Fowl Officer**

---

### Dog (D)

```text
. . . . .
. o o o .
. . # . .
. . . . .
. . . . .
```

- Promotes to: **Multi General**

---

### Drunken Elephant (DE)

```text
. . . . .
. o o o .
. o # o .
. . . . .
. . . . .
```

- Promotes to: **Crown Prince**

---

### Roaring Dog (DG)

```text
* . . * . . *
. \ . | . / .
. . \ | / . .
* - - # - - *
. . 1 | 1 . .
. 2 . | . 2 .
3 . . * . . 3
```

- Promotes to: **Lion Dog**

---

### Dragon Horse (DH)

```text
\ . . . /
. \ o / .
. o # o .
. / o \ .
/ . . . \
```

- Promotes to: **Horned Hawk**

---

### Dragon King (DK)

```text
. . | . .
. o | o .
- - # - -
. o | o .
. . | . .
```

- Promotes to: **Flying Eagle**

---

### Howling Dog (Left) (DL)

```text
. . | . .
. . | . .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Left Dog**

---

### Fire Demon (DM)

```text
. . 2 . .
. . 1 . .
- - # - -
. . 1 . .
. . 2 . .
```

- Promotes to: **Free Fire**

---

### Donkey (DO)

```text
. . 2 . .
. . 1 . .
2 1 # 1 2
. . 1 . .
. . 2 . .
```

- Promotes to: **Ceramic Dove**

---

### Howling Dog (Right) (DR)

```text
. . | . .
. . | . .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Right Dog**

---

### Dark Spirit (DS)

```text
. . . . .
. . o o .
. o # o .
. o o o .
. . . . .
```

- Promotes to: **Buddhist Spirit**

---

### Deva (DV)

```text
. . . . .
. o o . .
. o # o .
. o o o .
. . . . .
```

- Promotes to: **King of Teachings**

---

### Earth General (EA)

```text
. . . . .
. . o . .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **White Elephant**

---

### Enchanted Badger (EB)

```text
. . 2 . .
. . 1 . .
2 1 # 1 2
. . 1 . .
. . 2 . .
```

- Promotes to: **Ceramic Dove**

---

### Earth Chariot (EC)

```text
. . | . .
. . | . .
. o # o .
. . | . .
. . | . .
```

- Promotes to: **Reed Bird**

---

### Earth Dragon (ED)

```text
. . 2 . .
. o 1 o .
. . # . .
. / o \ .
/ . . . \
```

---

### Fierce Eagle (EG)

```text
2 . . . 2
. 1 o 1 .
. o # o .
. 1 . 1 .
2 . . . 2
```

- Promotes to: **Flying Eagle**

---

### Flying Eagle (EL)

```text
* . | . *
. \ | / .
- - # - -
. / | \ .
/ . | . \
```

- Promotes to: **Great Eagle**

---

### Eastern Barbarian (ES)

```text
. . 2 . .
. o 1 o .
. o # o .
. . 1 . .
. . 2 . .
```

- Promotes to: **Lion**

---

### Evil Wolf (EW)

```text
. . . . .
. o o o .
. o # o .
. . . . .
. . . . .
```

- Promotes to: **Poisonous Wolf**

---

### Fire General (F)

```text
. . 2 . .
. o 1 o .
. . # . .
. . 1 . .
. . 2 . .
```

- Promotes to: **Great General**

---

### Flying Cat (FC)

```text
* . . * . . *
. . . . . . .
. . . . . . .
* . . # . . *
. . o o o . .
. . . . . . .
. . . . . . .
```

- Promotes to: **Flying Chariot**

---

### Flying Dragon (FD)

```text
* . . . *
. . . . .
. . # . .
. . . . .
* . . . *
```

- Promotes to: **Dragon King**

---

### Free Eagle (FE)

```text
* . . * . . *
. * . * . * .
. . \ | / . .
* * - # - * *
. . / | \ . .
. * . * . * .
* . . * . . *
```

- Igui: captures adjacent piece without moving

---

### Fragrant Elephant (FG)

```text
2 . 2 . 2
. 1 1 1 .
2 1 # 1 2
. 1 1 1 .
2 . 2 . 2
```

- Promotes to: **Elephant King**

---

### Flying Horse (FH)

```text
2 . . . 2
. 1 . 1 .
. . # . .
. 1 . 1 .
2 . . . 2
```

- Promotes to: **Free King**

---

### Fire Dragon (FI)

```text
2 . | . 2
. 1 | 1 .
- - # - -
. 1 | 1 .
2 . | . 2
```

- Promotes to: **Kirin Master**

---

### Fierce Leopard (FL)

```text
. . . . .
. . o . .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Angle Mover**

---

### Forest Demon (FO)

```text
\ . 2 . /
. \ 1 / .
2 1 # 1 2
. . | . .
. . | . .
```

- Promotes to: **Thunder Runner**

---

### Free Pup (FP)

```text
\ . | . /
. \ | / .
2 1 # 1 2
. o | o .
. . | . .
```

- Promotes to: **Free Dog**

---

### Free Demon (FR)

```text
\ . 2 . /
. \ 1 / .
- - # - -
. / 1 \ .
/ . 2 . \
```

- Promotes to: **Free King**

---

### Flying Swallow (FS)

```text
\ . . . /
. \ . / .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Flying Chariot**

---

### Free Dream-eater (FT)

```text
. . | . .
. . | . .
2 1 # 1 2
. . | . .
. . | . .
```

- Promotes to: **Free King**

---

### Flying Goose (FY)

```text
. . . . .
. o o o .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Swallow's Wings**

---

### Gold General (G)

```text
. . . . .
. o o o .
. o # o .
. . o . .
. . . . .
```

- Promotes to: **Flying Chariot**

---

### Go-between (GB)

```text
. . . . .
. . o . .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Drunken Elephant**

---

### Gold Chariot (GC)

```text
. . | . .
. o | o .
2 1 # 1 2
. o | o .
. . | . .
```

- Promotes to: **Playful Parrot**

---

### Great Dragon (GD)

```text
\ . 2 . /
. \ 1 / .
. . # . .
. / 1 \ .
/ . 2 . \
```

- Promotes to: **Ancient Dragon**

---

### Great Standard (GE)

```text
\ . | . /
. \ | / .
- - # - -
. 1 | 1 .
2 . | . 2
```

---

### Great General (GG)

```text
. . . . .
. . . . .
. . # . .
. . . . .
. . . . .
```

- Range capture: N, NE, E, SE, S, SW, W, NW

---

### Gold Stag (GL)

```text
\ . . . /
. \ . / .
. . # . .
. 1 . 1 .
2 . . . 2
```

- Promotes to: **White Foal**

---

### Great Master (GM)

```text
* . . * . . *
. \ . | . / .
. . \ | / . .
3 2 1 # 1 2 3
. . 1 | 1 . .
. 2 . | . 2 .
3 . . | . . 3
```

---

### Wood General (GN)

```text
2 . . . 2
. 1 . 1 .
. . # . .
. . . . .
. . . . .
```

- Promotes to: **White Elephant**

---

### Gold Bird (GO)

```text
. . | . .
. . | . .
. . # . .
. 1 | 1 .
2 . | . 2
```

- Promotes to: **Free Bird**

---

### Great Dove (GR)

```text
\ . 2 . /
. \ 1 / .
2 1 # 1 2
. / 1 \ .
/ . 2 . \
```

- Promotes to: **Wooden Dove**

---

### Great Stag (GS)

```text
* . | . *
. . | . .
- - # - -
. 1 | 1 .
2 . | . 2
```

- Promotes to: **Free Stag**

---

### Great Turtle (GT)

```text
\ . * . /
. \ | / .
- - # - -
. / | \ .
/ . * . \
```

- Promotes to: **Spirit Turtle**

---

### Guardian of the Gods (GU)

```text
. . 2 . .
. . 1 . .
2 1 # 1 2
. . 1 . .
. . 2 . .
```

- Promotes to: **Heavenly Tetrarch**

---

### Horse General (H)

```text
. . 2 . .
. o 1 o .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Free Horse**

---

### Ram's-head Soldier (HE)

```text
\ . . . /
. \ . / .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Tiger Soldier**

---

### Horned Hawk (HF)

```text
\ . * . /
. \ | / .
- - # - -
. / | \ .
/ . | . \
```

- Promotes to: **Great Hawk**

---

### Hook Mover (HM)

```text
. . | . .
. . | . .
- - # - -
. . | . .
. . | . .
```

- Hook move (orth): slides, then optionally turns 90 degrees

---

### Horseman (HO)

```text
\ . | . /
. \ | / .
2 1 # 1 2
. . | . .
. . | . .
```

- Promotes to: **Cavalier**

---

### Running Horse (HR)

```text
\ . | . /
. \ | / .
. . # . .
. . o . .
* . . . *
```

- Promotes to: **Free Demon**

---

### Horse Soldier (HS)

```text
\ . | . /
. \ | / .
2 1 # 1 2
. . o . .
. . . . .
```

- Promotes to: **Running Horse**

---

### Iron General (I)

```text
. . . . .
. o o o .
. . # . .
. . . . .
. . . . .
```

- Promotes to: **White Elephant**

---

### King (K)

```text
2 . 2 . 2
. 1 1 1 .
2 1 # 1 2
. 1 1 1 .
2 . 2 . 2
```

---

### Kirin Master (KM)

```text
\ . . * . . /
. \ . | . / .
. . \ | / . .
3 2 1 # 1 2 3
. . / | \ . .
. / . | . \ .
/ . . * . . \
```

---

### Kirin (KR)

```text
. . . . .
. o o o .
* . # . *
. o o o .
. . . . .
```

- Promotes to: **Gold Bird**

---

### Incense Chariot (L)

```text
. . | . .
. . | . .
. . # . .
. . . . .
. . . . .
```

- Promotes to: **White Foal**

---

### Longbow Soldier (LB)

```text
2 . | . 2
. 1 | 1 .
2 1 # 1 2
. . o . .
. . . . .
```

- Promotes to: **Longbow General**

---

### Left Chariot (LC)

```text
. . | . /
. . | / .
. o # . .
. / . . .
/ . . . .
```

- Promotes to: **Left Iron Chariot**

---

### Lion Dog (LD)

```text
* . . * . . *
. \ . | . / .
. . \ | / . .
* - - # - - *
. . / | \ . .
. / . | . \ .
* . . * . . *
```

- Promotes to: **Great Elephant**

---

### Left Dragon (LE)

```text
. . . . /
. . . / .
2 1 # - -
. . . \ .
. . . . \
```

- Promotes to: **Vermillion Sparrow**

---

### Left General (LG)

```text
. . . . .
. . o o .
. . # o .
. . o o .
. . . . .
```

- Promotes to: **Left Army**

---

### Lion Hawk (LI)

```text
* * * * *
* \ o / *
* o # o *
* / o \ *
* * * * *
```

- Area move: up to 2 steps per turn
- Igui: captures adjacent piece without moving

---

### Little Turtle (LL)

```text
\ . * . /
. \ | / .
2 1 # 1 2
. / | \ .
/ . * . \
```

- Promotes to: **Treasure Turtle**

---

### Lion (LN)

```text
* * * * *
* o o o *
* o # o *
* o o o *
* * * * *
```

- Promotes to: **Furious Fiend**
- Area move: up to 2 steps per turn
- Igui: captures adjacent piece without moving

---

### Tengu (LO)

```text
\ . . . /
. \ . / .
. . # . .
. / . \ .
/ . . . \
```

- Hook move (diag): slides, then optionally turns 90 degrees

---

### Leopard Soldier (LP)

```text
2 . | . 2
. 1 | 1 .
2 1 # 1 2
. . o . .
. . . . .
```

- Promotes to: **Running Leopard**

---

### Little Standard (LS)

```text
2 . | . 2
. 1 | 1 .
- - # - -
. o | o .
. . | . .
```

- Promotes to: **Rear Standard**

---

### Left Tiger (LT)

```text
. . . . /
. o . / .
. . # - -
. o . \ .
. . . . \
```

- Promotes to: **Turtle-snake**

---

### Mountain General (M)

```text
2 . . . 2
. 1 o 1 .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Mount Tai**

---

### Mountain Hawk (MF)

```text
\ . * . /
. \ | / .
- - # - -
. 1 | 1 .
2 . | . 2
```

- Promotes to: **Horned Hawk**

---

### Side Monkey (MK)

```text
. . . . .
. o . o .
- - # - -
. . o . .
. . . . .
```

- Promotes to: **Side Soldier**

---

### Left Mountain Eagle (ML)

```text
* . | . /
. \ | / .
- - # - -
. / | 1 .
/ . | . 2
```

- Promotes to: **Flying Eagle**

---

### Right Mountain Eagle (MR)

```text
\ . | . *
. \ | / .
- - # - -
. 1 | \ .
2 . | . \
```

- Promotes to: **Flying Eagle**

---

### Mountain Stag (MS)

```text
2 . . . 2
. 1 o 1 .
2 1 # 1 2
. . 1 . .
. . 2 . .
```

- Promotes to: **Great Stag**

---

### Center Master (MT)

```text
* . * . *
. \ | / .
2 1 # 1 2
. 1 | 1 .
2 . * . 2
```

---

### Cassia Horse (N)

```text
. * . * .
. . . . .
. . # . .
. . . . .
. . . . .
```

- Promotes to: **Side Soldier**

---

### Northern Barbarian (NB)

```text
. . . . .
. o o o .
2 1 # 1 2
. . o . .
. . . . .
```

- Promotes to: **Wooden Dove**

---

### Neighboring King (NK)

```text
. . . . .
. o o o .
. o # o .
. . . . .
. . . . .
```

- Promotes to: **Front Standard**

---

### Fierce Wolf (NT)

```text
. . . . .
. o o o .
. . # . .
. o . o .
. . . . .
```

- Promotes to: **Bear's Eyes**

---

### Ox General (O)

```text
. . 2 . .
. o 1 o .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Free Ox**

---

### Ox Chariot (OC)

```text
. . | . .
. . | . .
. . # . .
. . . . .
. . . . .
```

- Promotes to: **Plodding Ox**

---

### Old Kite (OK)

```text
2 . . . 2
. 1 . 1 .
. o # o .
. 1 . 1 .
2 . . . 2
```

- Promotes to: **Tengu**

---

### Old Monkey (OM)

```text
. . . . .
. o . o .
. . # . .
. o o o .
. . . . .
```

- Promotes to: **Mountain Witch**

---

### Old Rat (OR)

```text
. . . . .
. . o . .
. . # . .
. o . o .
. . . . .
```

- Promotes to: **Mocking Bird**

---

### Ox Soldier (OS)

```text
\ . | . /
. \ | / .
2 1 # 1 2
. . o . .
. . . . .
```

- Promotes to: **Running Ox**

---

### Swooping Owl (OW)

```text
. . . . .
. . o . .
. . # . .
. o . o .
. . . . .
```

- Promotes to: **Cloud Eagle**

---

### Flying Ox (OX)

```text
\ . | . /
. \ | / .
. . # . .
. / | \ .
/ . | . \
```

- Promotes to: **Fire Ox**

---

### Pawn (P)

```text
. . . . .
. . o . .
. . # . .
. . . . .
. . . . .
```

- Promotes to: **Gold General**

---

### Peacock (PC)

```text
\ . . . /
. \ . / .
. . # . .
. 1 . 1 .
2 . . . 2
```

- Promotes to: **Tengu**
- Hook move (diag): slides, then optionally turns 90 degrees

---

### Peng Master (PE)

```text
* . . | . . *
. \ . | . / .
. . \ | / . .
3 2 1 # 1 2 3
. . 1 | 1 . .
. 2 . | . 2 .
3 . . | . . 3
```

---

### Pup General (PG)

```text
. . 2 . .
. o 1 o .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Free Pup**

---

### Phoenix (PH)

```text
* . . . *
. . o . .
. o # o .
. . o . .
* . . . *
```

- Promotes to: **Gold Bird**

---

### Pig General (PI)

```text
2 . . . 2
. 1 . 1 .
. . # . .
. . 1 . .
. . 2 . .
```

- Promotes to: **Free Pig**

---

### Phoenix Master (PM)

```text
* . . | . . *
. \ . | . / .
. . \ | / . .
3 2 1 # 1 2 3
. . / | \ . .
. / . | . \ .
/ . . | . . \
```

---

### Prancing Stag (PR)

```text
2 . . . 2
. 1 . 1 .
. o # o .
. 1 . 1 .
2 . . . 2
```

- Promotes to: **Square Mover**

---

### Poisonous Serpent (PS)

```text
. . 2 . .
. o 1 o .
2 1 # 1 2
. . o . .
. . . . .
```

- Promotes to: **Hook Mover**

---

### Free King (Q)

```text
\ . | . /
. \ | / .
- - # - -
. / | \ .
/ . | . \
```

- Promotes to: **Great General**

---

### Flying Chariot (R)

```text
. . | . .
. . | . .
- - # - -
. . | . .
. . | . .
```

- Promotes to: **Dragon King**

---

### Rain Dragon (RA)

```text
. . . . .
. o o o .
- - # - -
. / | \ .
/ . | . \
```

- Promotes to: **Great Dragon**

---

### Rushing Bird (RB)

```text
. . 2 . .
. o 1 o .
. o # o .
. o . o .
. . . . .
```

- Promotes to: **Free Demon**

---

### Right Chariot (RC)

```text
\ . | . .
. \ | . .
. . # o .
. . . \ .
. . . . \
```

- Promotes to: **Right Iron Chariot**

---

### Reclining Dragon (RD)

```text
. . . . .
. . o . .
. o # o .
. . o . .
. . . . .
```

- Promotes to: **Great Dragon**

---

### River General (RE)

```text
. . 2 . .
. o 1 o .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Huai River**

---

### Right General (RG)

```text
. . . . .
. o o . .
. o # . .
. o o . .
. . . . .
```

- Promotes to: **Right Army**

---

### Running Chariot (RH)

```text
\ . | . /
. \ | / .
. o # o .
. . | . .
. . | . .
```

- Promotes to: **Cannon Chariot**

---

### Right Dragon (RI)

```text
\ . . . .
. \ . . .
- - # 1 2
. / . . .
/ . . . .
```

- Promotes to: **Blue Dragon**

---

### Running Stag (RN)

```text
\ . . . /
. \ . / .
- - # - -
. . 1 . .
. . 2 . .
```

- Promotes to: **Free Stag**

---

### Flying General (RO)

```text
. . . . .
. . . . .
. . # . .
. . . . .
. . . . .
```

- Promotes to: **Flying Crocodile**
- Range capture: N, E, S, W

---

### Running Pup (RP)

```text
. . | . .
. . | . .
. . # . .
. / o . .
/ . . . .
```

- Promotes to: **Free Leopard**

---

### Running Rabbit (RR)

```text
\ . | . /
. \ | / .
. . # . .
. o o o .
. . . . .
```

- Promotes to: **Treacherous Fox**

---

### Rear Standard (RS)

```text
2 . | . 2
. 1 | 1 .
- - # - -
. 1 | 1 .
2 . | . 2
```

- Promotes to: **Center Standard**

---

### Running Tiger (RT)

```text
. . | . .
. . | . .
. . # . .
. . o \ .
. . . . \
```

- Promotes to: **Free Tiger**

---

### Running Serpent (RU)

```text
. . | . .
. . | . .
. . # . .
. / o . .
/ . . . .
```

- Promotes to: **Free Serpent**

---

### Reverse Chariot (RV)

```text
. . | . .
. . | . .
. . # . .
. . | . .
. . | . .
```

- Promotes to: **Whale**

---

### Running Wolf (RW)

```text
\ . . . /
. \ o / .
- - # - -
. . . . .
. . . . .
```

- Promotes to: **Free Wolf**

---

### Silver General (S)

```text
. . . . .
. o o o .
. . # . .
. o . o .
. . . . .
```

- Promotes to: **Vertical Mover**

---

### Side Boar (SA)

```text
. . . . .
. o o o .
- - # - -
. o o o .
. . . . .
```

- Promotes to: **Free Bird**

---

### Crossbow Soldier (SC)

```text
2 . 2 . 2
. 1 1 1 .
2 1 # 1 2
. . o . .
. . . . .
```

- Promotes to: **Crossbow General**

---

### Front Standard (SD)

```text
2 . | . 2
. 1 | 1 .
- - # - -
. 1 | 1 .
2 . | . 2
```

- Promotes to: **Great Standard**

---

### Sword Soldier (SE)

```text
. . . . .
. o . o .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Sword General**

---

### Side Flyer (SF)

```text
. . . . .
. o . o .
- - # - -
. o . o .
. . . . .
```

- Promotes to: **Side Dragon**

---

### Stone General (SG)

```text
. . . . .
. o . o .
. . # . .
. . . . .
. . . . .
```

- Promotes to: **White Elephant**

---

### Side Dragon (SI)

```text
. . | . .
. . | . .
- - # - -
. . . . .
. . . . .
```

- Promotes to: **Running Dragon**

---

### Side Soldier (SL)

```text
. . 2 . .
. . 1 . .
- - # - -
. . o . .
. . . . .
```

- Promotes to: **Water Ox**

---

### Side Mover (SM)

```text
. . . . .
. . o . .
- - # - -
. . o . .
. . . . .
```

- Promotes to: **Free Bird**

---

### Coiled Serpent (SN)

```text
. . . . .
. . o . .
. . # . .
. o o o .
. . . . .
```

- Promotes to: **Coiled Dragon**

---

### Soldier (SO)

```text
\ . | . /
. \ | / .
. o # o .
. . | . .
. . | . .
```

- Promotes to: **Cavalier**

---

### Spear Soldier (SP)

```text
. . | . .
. . | . .
. o # o .
. . o . .
. . . . .
```

- Promotes to: **Spear General**

---

### Square Mover (SQ)

```text
. . | . .
. . | . .
- - # - -
. . | . .
. . | . .
```

- Promotes to: **Strong Chariot**

---

### Silver Rabbit (SR)

```text
2 . . . 2
. 1 . 1 .
. . # . .
. / . \ .
/ . . . \
```

- Promotes to: **Whale**

---

### Side Serpent (SS)

```text
. . 2 . .
. . 1 . .
- - # - -
. . o . .
. . . . .
```

- Promotes to: **Great Shark**

---

### Strutting Crow (ST)

```text
. . . . .
. . o . .
. . # . .
. o . o .
. . . . .
```

- Promotes to: **Flying Hawk**

---

### Southern Barbarian (SU)

```text
. . 2 . .
. o 1 o .
. o # o .
. . 1 . .
. . 2 . .
```

- Promotes to: **Gold Bird**

---

### Silver Chariot (SV)

```text
2 . | . 2
. 1 | 1 .
. . # . .
. o | o .
. . | . .
```

- Promotes to: **Goose Wing**

---

### Swallow's Wings (SW)

```text
. . . . .
. . o . .
- - # - -
. . o . .
. . . . .
```

- Promotes to: **Gliding Swallow**

---

### Side Ox (SX)

```text
. . . . .
. . . o .
- - # - -
. o . . .
. . . . .
```

- Promotes to: **Flying Ox**

---

### Tile General (T)

```text
. . . . .
. o . o .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **White Elephant**

---

### Tile Chariot (TC)

```text
. . | . .
. . | o .
. . # . .
. o | . .
. . | . .
```

- Promotes to: **Running Tile**

---

### Turtle Dove (TD)

```text
2 . . . 2
. 1 . 1 .
. o # o .
. . o . .
. . . . .
```

- Promotes to: **Great Dove**

---

### Treacherous Fox (TF)

```text
\ . . * . . /
. * . * . * .
. . \ | / . .
* * - # - * *
. . / | \ . .
. * . * . * .
/ . . * . . \
```

- Promotes to: **Mountain Crane**

---

### Fierce Tiger (TG)

```text
. . | . .
. . | . .
. . # . .
. . . . .
. . . . .
```

- Promotes to: **Great Tiger**

---

### Turtle-snake (TS)

```text
. . . . /
. o o / .
. o # o .
. / o o .
/ . . . .
```

- Promotes to: **Divine Turtle**

---

### Right Tiger (TT)

```text
\ . . . .
. \ . o .
- - # . .
. / . o .
/ . . . .
```

- Promotes to: **White Tiger**

---

### Fierce Bear (VB)

```text
2 . . . 2
. 1 o 1 .
. o # o .
. . . . .
. . . . .
```

- Promotes to: **Great Bear**

---

### Fierce Dragon (VD)

```text
. . 2 . .
. . 1 . .
2 1 # 1 2
. . 1 . .
. . 2 . .
```

- Promotes to: **Great Dragon**
- Range capture: NE, SE, SW, NW

---

### Vertical Bear (VE)

```text
. . | . .
. . | . .
2 1 # 1 2
. . o . .
. . . . .
```

- Promotes to: **Free Bear**

---

### Vice General (VG)

```text
. . * . .
. . . . .
* . # . *
. . . . .
. . * . .
```

- Promotes to: **Great General**
- Range capture: NE, SE, SW, NW

---

### Vertical Horse (VH)

```text
. . | . .
. o | o .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Dragon Horse**

---

### Vermillion Sparrow (VI)

```text
\ . . . .
. \ o o .
. o # o .
. o o \ .
. . . . \
```

- Promotes to: **Divine Sparrow**

---

### Vertical Leopard (VL)

```text
. . | . .
. o | o .
. o # o .
. . o . .
. . . . .
```

- Promotes to: **Great Leopard**

---

### Vertical Mover (VM)

```text
. . | . .
. . | . .
. o # o .
. . | . .
. . | . .
```

- Promotes to: **Flying Ox**

---

### Fierce Ox (VO)

```text
\ . . . /
. \ o / .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Flying Ox**

---

### Vertical Pup (VP)

```text
. . | . .
. . | . .
. . # . .
. o o o .
. . . . .
```

- Promotes to: **Leopard King**

---

### Vertical Soldier (VR)

```text
. . 2 . .
. . 1 . .
- - # - -
. . o . .
. . . . .
```

- Promotes to: **Chariot Soldier**

---

### Fierce Stag (VS)

```text
. . . . .
. o o o .
. . # . .
. o . o .
. . . . .
```

- Promotes to: **Rushing Boar**

---

### Vertical Tiger (VT)

```text
. . | . .
. . | . .
. . # . .
. . 1 . .
. . 2 . .
```

- Promotes to: **Free Tiger**

---

### Vertical Wolf (VW)

```text
. . | . .
. . | . .
. o # o .
. . 1 . .
. . 2 . .
```

- Promotes to: **Running Wolf**

---

### Whale (W)

```text
\ . . . /
. \ . / .
. . # . .
. / | \ .
/ . | . \
```

- Promotes to: **Great Whale**

---

### Water Dragon (WA)

```text
2 . | . 2
. 1 | 1 .
- - # - -
. 1 | 1 .
2 . | . 2
```

- Promotes to: **Phoenix Master**

---

### Water Ox (WB)

```text
. . | . .
. o | o .
2 1 # 1 2
. o | o .
. . | . .
```

- Promotes to: **Great Dream-eater**

---

### Wood Chariot (WC)

```text
. . | . .
. o | . .
. . # . .
. . | o .
. . | . .
```

- Promotes to: **Wind Snapping Turtle**

---

### Wind Dragon (WD)

```text
\ . . . /
. \ . / .
- - # - -
. o . \ .
. . . . \
```

- Promotes to: **Free Dragon**

---

### White Elephant (WE)

```text
2 . 2 . 2
. 1 1 1 .
2 1 # 1 2
. 1 1 1 .
2 . 2 . 2
```

- Promotes to: **Elephant King**

---

### Side Wolf (WF)

```text
. . . . .
. o . . .
- - # - -
. . . o .
. . . . .
```

- Promotes to: **Free Wolf**

---

### Water General (WG)

```text
2 . . . 2
. 1 o 1 .
. . # . .
. . o . .
. . . . .
```

- Promotes to: **Vice General**

---

### White Foal (WH)

```text
\ . | . /
. \ | / .
. . # . .
. . | . .
. . | . .
```

- Promotes to: **Great Foal**

---

### Wind Horse (WI)

```text
. . | . .
. o | o .
. . # . .
. . 1 . .
. . 2 . .
```

- Promotes to: **Heavenly Horse**

---

### Woodland Demon (WL)

```text
\ . | . /
. \ | / .
2 1 # 1 2
. 1 | 1 .
2 . | . 2
```

- Promotes to: **Stone Peng**

---

### Wind General (WN)

```text
. . 2 . .
. o 1 o .
2 1 # 1 2
. . o . .
. . . . .
```

- Promotes to: **Fierce Wind**

---

### Wooden Dove (WO)

```text
* . . . . . *
. \ . 2 . / .
. . \ 1 / . .
. 2 1 # 1 2 .
. . / 1 \ . .
. / . 2 . \ .
* . . . . . *
```

---

### Sumo Wrestler (WR)

```text
2 . . . 2
. 1 . 1 .
. . # . .
. 1 . 1 .
2 . . . 2
```

- Promotes to: **Heavenly Tetrarch**

---

### Western Barbarian (WS)

```text
. . 2 . .
. o 1 o .
. o # o .
. . 1 . .
. . 2 . .
```

- Promotes to: **Lion Dog**

---

### White Tiger (WT)

```text
\ . 2 . .
. \ 1 . .
- - # - -
. . 1 . .
. . 2 . .
```

- Promotes to: **Divine Tiger**

---

### Nature Spirit (YA)

```text
. . . . .
. o . o .
2 1 # 1 2
. . o . .
. . . . .
```

- Promotes to: **Heavenly Tetrarch**

---

## Promoted-Only Pieces (92)

### Ancient Dragon (+ADR)

```text
\ . | . /
. \ | / .
. . # . .
. / | \ .
/ . | . \
```

---

### Beast Bird (+BBR)

```text
\ . | . /
. \ | / .
2 1 # 1 2
. / 1 \ .
/ . 2 . \
```

---

### Bear's Eyes (+BEY)

```text
. . . . .
. o o o .
. o # o .
. o o o .
. . . . .
```

---

### Buddhist Spirit (+BSP)

```text
* * * * *
* \ | / *
* - # - *
* / | \ *
* * * * *
```

- Area move: up to 2 steps per turn
- Igui: captures adjacent piece without moving

---

### Crossbow General (+CBG)

```text
2 . | . 2
. 1 | 1 .
2 1 # 1 2
. . 1 . .
. . 2 . .
```

---

### Cannon Chariot (+CCH)

```text
\ . | . /
. \ | / .
. o # o .
. . | . .
. . | . .
```

---

### Coiled Dragon (+CDR)

```text
. . | . .
. . | . .
. . # . .
. / | \ .
/ . | . \
```

---

### Copper Elephant (+CEL)

```text
. . | . .
. o | o .
. . # . .
. o | o .
. . | . .
```

---

### Cannon General (+CNG)

```text
\ . | . /
. \ | / .
2 1 # 1 2
. . 1 . .
. . 2 . .
```

---

### Cavalier (+CVL)

```text
\ . | . /
. \ | / .
- - # - -
. . | . .
. . | . .
```

---

### Divine Dragon (+DDR)

```text
. . | . /
. . | / .
2 1 # - -
. . | . .
. . | . .
```

---

### Divine Sparrow (+DSP)

```text
\ . . . .
. \ o o .
. o # o .
. / o \ .
/ . . . \
```

---

### Divine Tiger (+DTI)

```text
\ . | . .
. \ | . .
- - # - -
. . 1 . .
. . 2 . .
```

---

### Divine Turtle (+DTU)

```text
. . . . /
. o o / .
. o # o .
. / o \ .
/ . . . \
```

---

### Elephant King (+EKI)

```text
\ . 2 . /
. \ 1 / .
2 1 # 1 2
. / 1 \ .
/ . 2 . \
```

---

### Free Bear (+FBR)

```text
. . . . /
. o o / .
. o # o .
. / o \ .
/ . . . \
```

---

### Free Boar (+FB) 奔猪

```text
\ . | . /
. \ | / .
- - # - -
. . o . .
. . . . .
```

---

### Free Chicken (+FCK)

```text
. . | . .
. o | o .
. o # o .
. . . . .
. . . . .
```

---

### Flying Crocodile (+FCR)

```text
2 . . . 2
. 1 . 1 .
. . # . .
. 1 . 1 .
2 . . . 2
```

- Range capture: N, E, S, W

---

### Free Dog (+FDG)

```text
\ . | . /
. \ | / .
2 1 # 1 2
. o | o .
. . | . .
```

---

### Free Dragon (+FDR)

```text
\ . . . /
. \ . / .
- - # - -
. / | \ .
/ . | . \
```

---

### Furious Fiend (+FFI)

```text
* * * * *
* 1 1 1 *
* 1 # 1 *
* 1 1 1 *
* * * * *
```

- Area move: up to 2 steps per turn
- Igui: captures adjacent piece without moving

---

### Free Fire (+FFR)

```text
. . 2 . .
. . 1 . .
- - # - -
. . 1 . .
. . 2 . .
```

---

### Flying Hawk (+FHK)

```text
\ . . . /
. \ o / .
. . # . .
. / . \ .
/ . . . \
```

---

### Free Horse (+FHO)

```text
\ . | . /
. \ | / .
- - # - -
. . . . .
. . . . .
```

---

### Free Leopard (+FLE)

```text
\ . . . /
. \ . / .
- - # - -
. . | . .
. . | . .
```

---

### Fowl (+FOW)

```text
2 . 2 . 2
. 1 1 1 .
2 1 # 1 2
. 1 . 1 .
2 . . . 2
```

---

### Free Ox (+FOX)

```text
\ . | . /
. \ | / .
- - # - -
. . . . .
. . . . .
```

---

### Fire Ox (+FOI)

```text
. . | . .
. . | . .
. o # o .
. . | . .
. . | . .
```

---

### Free Pig (+FPI)

```text
\ . | . /
. \ | / .
- - # - -
. . . . .
. . . . .
```

---

### Free Serpent (+FSE)

```text
. . | . .
. . | . .
. . # . .
. / | \ .
/ . | . \
```

---

### Flying Stag (+FSG)

```text
. . | . .
. o | o .
. o # o .
. o | o .
. . | . .
```

---

### Free Stag (+FST)

```text
\ . | . /
. \ | / .
- - # - -
. / | \ .
/ . | . \
```

---

### Free Tiger (+FTI)

```text
. . | . .
. . | . .
. . # . .
. . 1 . .
. . 2 . .
```

---

### Fierce Wind (+FWI)

```text
. . | . .
. o | o .
. . # . .
. . 1 . .
. . 2 . .
```

---

### Free Wolf (+FWO)

```text
\ . | . /
. \ | / .
- - # - -
. . . . .
. . . . .
```

---

### Great Bear (+GBR)

```text
\ . | . /
. \ | / .
. o # o .
. . o . .
. . . . .
```

---

### Great Dream-eater (+GDE)

```text
\ . . | . . /
. \ . | . / .
. . \ | / . .
* - - # - - *
. . / | \ . .
. / . | . \ .
/ . . | . . \
```

---

### Great Eagle (+GEA)

```text
* . | . *
. \ | / .
- - # - -
. / | \ .
/ . | . \
```

---

### Great Elephant (+GEL)

```text
2 . | . 2
. 1 | 1 .
- - # - -
. . | . .
. . | . .
```

---

### Great Foal (+GFO)

```text
\ . | . /
. \ | / .
. . # . .
. . | . .
. . | . .
```

---

### Great Hawk (+GHK)

```text
\ . * . /
. \ | / .
- - # - -
. / | \ .
/ . | . \
```

---

### Great Shark (+GSH)

```text
2 . | . 2
. 1 | 1 .
- - # - -
. 1 | 1 .
2 . | . 2
```

---

### Gliding Swallow (+GSW)

```text
. . | . .
. . | . .
- - # - -
. . | . .
. . | . .
```

---

### Great Tiger (+GTI)

```text
. . . . .
. . o . .
- - # - -
. . | . .
. . | . .
```

---

### Great Leopard (+GLE)

```text
2 . | . 2
. 1 | 1 .
2 1 # 1 2
. . o . .
. . . . .
```

---

### Great Whale (+GWH)

```text
\ . . . /
. \ . / .
. . # . .
. / | \ .
/ . | . \
```

---

### Goose Wing (+GWI)

```text
. . . . .
. o o o .
2 1 # 1 2
. o o o .
. . . . .
```

---

### Heavenly Horse (+HHO)

```text
. * | * .
. . | . .
. . # . .
. . . . .
. * . * .
```

---

### Huai River (+HRI)

```text
. . . . .
. . o . .
- - # - -
. . o . .
. . . . .
```

---

### Heavenly Tetrarch (+HT)

```text
2 . 2 . 2
. 1 1 1 .
2 1 # 1 2
. 1 1 1 .
2 . 2 . 2
```

---

### Heavenly Tetrarch King (+HTK)

```text
* . * . *
. \ | / .
* - # - *
. / | \ .
* . * . *
```

---

### King of Teachings (+KTE)

```text
* . . * . . *
. \ . | . / .
. . \ | / . .
* - - # - - *
. . / | \ . .
. / . | . \ .
* . . * . . *
```

---

### Left Army (+LAR)

```text
\ . . . .
. \ o o .
- - # o .
. / o o .
/ . . . .
```

---

### Longbow General (+LBG)

```text
\ . | . /
. \ | / .
2 1 # 1 2
. . | . .
. . | . .
```

---

### Left Dog (+LDG)

```text
. . | . .
. . | . .
. . # . .
. . o \ .
. . . . \
```

---

### Left Iron Chariot (+LIC)

```text
. . . . /
. . . / .
. o # . .
. / . \ .
/ . . . \
```

---

### Leopard King (+LKI)

```text
2 . 2 . 2
. 1 1 1 .
2 1 # 1 2
. 1 1 1 .
2 . 2 . 2
```

---

### Mocking Bird (+MBI)

```text
\ . | . /
. \ | / .
. . # . .
. . | . .
. . | . .
```

---

### Mountain Crane (+MCR)

```text
* . . * . . *
. * . * . * .
. . \ | / . .
* * - # - * *
. . / | \ . .
. * . * . * .
* . . * . . *
```

---

### Mount Tai (+MTA)

```text
\ . 2 . /
. \ 1 / .
2 1 # 1 2
. / . \ .
/ . . . \
```

---

### Multi General (+MUG)

```text
\ . | . /
. \ | / .
. . # . .
. . | . .
. . | . .
```

---

### Mountain Witch (+MWI)

```text
\ . . . /
. \ . / .
. . # . .
. / | \ .
/ . | . \
```

---

### Plodding Ox (+POX)

```text
. . | . .
. o | o .
. . # . .
. o | o .
. . | . .
```

---

### Playful Parrot (+PPR)

```text
2 . | . 2
. 1 | 1 .
2 1 # 1 2
. 1 | 1 .
2 . | . 2
```

---

### Poisonous Wolf (+PWO)

```text
. . . . .
. o o o .
. o # o .
. o o o .
. . . . .
```

---

### Right Army (+RAR)

```text
. . . . /
. o o / .
. o # - -
. o o \ .
. . . . \
```

---

### Reed Bird (+RBI)

```text
. . | . .
. . | . .
2 1 # 1 2
. 1 | 1 .
2 . | . 2
```

---

### Running Boar (+RBO)

```text
. . . . .
. o o o .
- - # - -
. o o o .
. . . . .
```

---

### Right Dog (+RDG)

```text
. . | . .
. . | . .
. . # . .
. / o . .
/ . . . .
```

---

### Rain Demon (+RDM)

```text
. . 2 . .
. . 1 . .
2 1 # 1 2
. 1 | 1 .
2 . | . 2
```

---

### Running Dragon (+RDR)

```text
\ . | . /
. \ | / .
- - # - -
. . 1 . .
. . 2 . .
```

---

### Raiding Hawk (+RHK)

```text
. . | . .
. o | o .
. o # o .
. . . . .
. . . . .
```

---

### Right Iron Chariot (+RIC)

```text
\ . . . .
. \ . . .
. . # o .
. / . \ .
/ . . . \
```

---

### Running Leopard (+RLE)

```text
. . | . .
. . | . .
2 1 # 1 2
. . 1 . .
. . 2 . .
```

---

### Running Ox (+ROX)

```text
\ . | . /
. \ | / .
- - # - -
. . 1 . .
. . 2 . .
```

---

### Running Tile (+RTI)

```text
. . | . .
. . | . .
2 1 # 1 2
. . | . .
. . | . .
```

---

### Rushing Boar (+RUB)

```text
. . . . .
. o o o .
. o # o .
. . . . .
. . . . .
```

---

### Strong Bear (+SBR)

```text
\ . | . /
. \ | / .
- - # - -
. . 1 . .
. . 2 . .
```

---

### Strong Chariot (+SCH)

```text
\ . | . /
. \ | / .
- - # - -
. . | . .
. . | . .
```

---

### Strong Eagle (+SEA)

```text
\ . | . /
. \ | / .
- - # - -
. / | \ .
/ . | . \
```

---

### Stone Peng (+SPE)

```text
\ . . . /
. \ . / .
2 1 # 1 2
. / . \ .
/ . . . \
```

---

### Spear General (+SPG)

```text
. . | . .
. . | . .
2 1 # 1 2
. . 1 . .
. . 2 . .
```

---

### Spirit Turtle (+SPT)

```text
\ . . * . . /
. \ . | . / .
. . \ | / . .
* - - # - - *
. . / | \ . .
. / . | . \ .
/ . . * . . \
```

---

### Sword General (+SWG)

```text
2 . 2 . 2
. 1 1 1 .
. . # . .
. . o . .
. . . . .
```

---

### Thunder Runner (+THR)

```text
\ . | . /
. \ | / .
2 1 # 1 2
. . 1 . .
. . 2 . .
```

---

### Treasure Turtle (+TRT)

```text
\ . * . /
. \ | / .
* - # - *
. / | \ .
/ . * . \
```

---

### Tiger Soldier (+TSO)

```text
\ . 2 . /
. \ 1 / .
. . # . .
. . o . .
. . . . .
```

---

### Walking Heron (+WHR)

```text
2 . | . 2
. 1 | 1 .
2 1 # 1 2
. . | . .
. . | . .
```

---

### Wizard Stork (+WST)

```text
\ . . . /
. \ . / .
- - # - -
. . | . .
. . | . .
```

---

### Wind Snapping Turtle (+WST2)

```text
2 . | . 2
. 1 | 1 .
. . # . .
. . | . .
. . | . .
```

---

### Free Bird (+FBI)

```text
. . | . .
. . | . .
- - # - -
. 1 | 1 .
2 . | . 2
```

---
