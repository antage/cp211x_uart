error_chain! {
    foreign_links {
        HidError(::hid::Error);
    }

    errors {
        WriteTimeout {
            description("write operation is time out")
            display("write operation is time out")
        }
    }
}