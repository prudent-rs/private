use restricted::*;
//use private::prelude::*;

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
        def_const!(B: bool = true);

        let _ = at_const!(B);

        {
            def_const_direct!(U: u8 = 1);
            let _ = at_const!(U);
            //@TODO add token(s):
            let _ = U!(.);
        }
    }
    {
        def_static!(B: bool = true);

        let _ = at_static!(B);

        {
            def_static_direct!(U: u8 = 1);
            let _ = at_static!(U);
            //@TODO add token(s):
            let _ = U!(.);
        }
    }
    /* */
    //bufo_bufo_private_ident_here_dimvxevsdmqmbnuhyptltyqdlnafhdbg= 0;
}
