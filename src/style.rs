pub fn load_css() {
    let provider = gtk4::CssProvider::new();

    provider.load_from_data(
        "
	    .sidebar-heading {
	        font-size: 18px;
	        font-weight: bold;
	    }
        .pathbar {
            margin: 5px;
        }
        .footer-bar {
            background-color: #f0f0f0;
            border-top: 1px solid #d0d0d0;
            padding: 5px 10px;
            font-size: 12px;
        }
        .footer-label {
            margin: 0 10px;
        }
	",
    );

    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("No display found"),
        &provider,
        900,
    );
}
