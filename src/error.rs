error_chain! {
    foreign_links {
        HidError(::hidapi::HidError);
    }

    errors {
        WriteTimeout {
            description("write operation is time out")
            display("write operation is time out")
        }
    }
}
