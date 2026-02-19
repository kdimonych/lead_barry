use static_cell::StaticCell;

pub trait InputAction {}
pub struct OkAction;
impl InputAction for OkAction {}
impl From<OkAction> for Action {
    fn from(action: OkAction) -> Self {
        Action::Ok(action)
    }
}
pub struct NextAction;
impl InputAction for NextAction {}
impl From<NextAction> for Action {
    fn from(action: NextAction) -> Self {
        Action::Next(action)
    }
}
pub struct PreviousAction;
impl InputAction for PreviousAction {}
impl From<PreviousAction> for Action {
    fn from(action: PreviousAction) -> Self {
        Action::Previous(action)
    }
}

pub struct UpAction;
impl InputAction for UpAction {}
impl From<UpAction> for Action {
    fn from(action: UpAction) -> Self {
        Action::Up(action)
    }
}
pub struct DownAction;
impl InputAction for DownAction {}
impl From<DownAction> for Action {
    fn from(action: DownAction) -> Self {
        Action::Down(action)
    }
}
pub struct LeftAction;
impl InputAction for LeftAction {}
impl From<LeftAction> for Action {
    fn from(action: LeftAction) -> Self {
        Action::Left(action)
    }
}
pub struct RightAction;
impl InputAction for RightAction {}
impl From<RightAction> for Action {
    fn from(action: RightAction) -> Self {
        Action::Right(action)
    }
}
pub struct F1Action;
impl InputAction for F1Action {}
impl From<F1Action> for Action {
    fn from(action: F1Action) -> Self {
        Action::F1(action)
    }
}
pub struct F2Action;
impl InputAction for F2Action {}
impl From<F2Action> for Action {
    fn from(action: F2Action) -> Self {
        Action::F2(action)
    }
}
pub struct F3Action;
impl InputAction for F3Action {}
impl From<F3Action> for Action {
    fn from(action: F3Action) -> Self {
        Action::F3(action)
    }
}
pub struct F4Action;
impl InputAction for F4Action {}
impl From<F4Action> for Action {
    fn from(action: F4Action) -> Self {
        Action::F4(action)
    }
}

pub struct ActionPallet {
    pub ok: OkAction,
    pub next: NextAction,
    pub previous: PreviousAction,
    pub up: UpAction,
    pub down: DownAction,
    pub left: LeftAction,
    pub right: RightAction,
    pub f1: F1Action,
    pub f2: F2Action,
    pub f3: F3Action,
    pub f4: F4Action,
}

impl ActionPallet {
    const fn new() -> Self {
        Self {
            ok: OkAction,
            next: NextAction,
            previous: PreviousAction,
            up: UpAction,
            down: DownAction,
            left: LeftAction,
            right: RightAction,
            f1: F1Action,
            f2: F2Action,
            f3: F3Action,
            f4: F4Action,
        }
    }
}

pub enum Action {
    Ok(OkAction),
    Next(NextAction),
    Previous(PreviousAction),
    Up(UpAction),
    Down(DownAction),
    Left(LeftAction),
    Right(RightAction),
    F1(F1Action),
    F2(F2Action),
    F3(F3Action),
    F4(F4Action),
}
impl Action {}

/// Initialize the action pallet and return a mutable reference to it.
/// This should be called once at the start of the program.
/// The action pallet is used to manage the actions as unique
/// resources that can be binded to specific buttons / button combinations button actions.
pub fn input_action_pallet() -> &'static mut ActionPallet {
    static ACTION_PALLET: StaticCell<ActionPallet> = StaticCell::new();
    ACTION_PALLET.init_with(ActionPallet::new)
}
