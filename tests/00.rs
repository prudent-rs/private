use restricted::*;
//use private::prelude::*;

macro_rules! defi_consti {
    ( $short_name:ident:$ty:ty = $value:expr ) => {
        #[doc = "Happy DI"]
        const $short_name: $ty = $value;
    };
}
defi_consti!(DI : bool = false);
const DDI: bool = DI;

def_const!(B: bool = true);
def_const_direct!(U: u8 = 1);

// @TODO have compile_fail test where a submodule from a different file fails
//
//mod sub;

// #[must_use]
fn f() {
    #![allow(unused)]

    //#[deprecated]
    macro_rules! unused {
        () => {
            let unused = ();
        };
    }
    macro_rules! allowed_unused {
        () => {
            #[allow(unused)]
            let unused = ();
        };
    }

    {
        #![deny(unused)]
        let _ok_to_be_unused = ();

        // fails to compile - OK:
        //
        //let x = ();

        {
            #![allow(unused)]
            unused!();
        }

        // ok
        allowed_unused!();
    }
    {
        def_const!(B2: bool = true);

        let _ = at_const!(B2);

        {
            def_const_direct!(U2: u8 = 1);
            let _ = at_const!(U2);
            //@TODO add token(s):
            let _ = U2!(.);
        }
    }
    {
        def_static!(B3: bool = true);

        let _ = at_static!(B3);

        {
            def_static_direct!(U3: u8 = 1);
            let _ = at_static!(U3);
            //@TODO add token(s):
            let _ = U3!(.);
        }
    }
    /* */
    //bufo_bufo_private_ident_here_dimvxevsdmqmbnuhyptltyqdlnafhdbg= 0;
}
