bitfield!{
    #[derive(Copy, Clone)]
    pub struct Status(u8);
    impl Debug;
    pub sprite_overflow, set_sprite_overflow:        5;
    pub sprite_zero_hit, set_sprite_zero_hit:        6;
    pub vblank,          set_vblank:                 7;
    pub get,             _:                      7,  0; // Full data
}
