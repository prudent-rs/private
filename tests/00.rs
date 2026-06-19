use private::{def_let, def_mut, at_let, at_mut};

fn f() {
    #![allow(unused)]
    // *local* lints don't apply
    //
    //#[allow(unused)]
    def_let!(ident_here);
    at_let!(ident_here) = 1;
    /* */
    def_let!(ident_here@::bufo::bufo :bool);
    at_let!(ident_here@::bufo::bufo) = true;
    /* */
    //bufo_bufo_private_ident_here_dimvxevsdmqmbnuhyptltyqdlnafhdbg= 0;
}
