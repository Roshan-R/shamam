use gtk::glib;
use gtk::prelude::*;

pub fn set_image_from_bytes(image: &gtk::Image, data: Vec<u8>) -> Option<gdk_pixbuf::Pixbuf> {
    // Create a Pixbuf from the byte data
    let loader = gdk_pixbuf::PixbufLoader::new();
    loader.write(&data).unwrap();
    loader.close().unwrap();

    loader.pixbuf()
}
