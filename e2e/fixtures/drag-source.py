"""Ventana GTK que arrastra archivos, como un gestor de archivos. Uso: drag-source.py X Y archivo…"""
import sys
from pathlib import Path

import gi

gi.require_version('Gdk', '3.0')
gi.require_version('Gtk', '3.0')
from gi.repository import Gdk, Gtk

x, y, *files = sys.argv[1:]
win = Gtk.Window(type=Gtk.WindowType.POPUP)
win.set_default_size(120, 60)
box = Gtk.EventBox()
box.add(Gtk.Label(label='arrastra'))
box.drag_source_set(Gdk.ModifierType.BUTTON1_MASK, [], Gdk.DragAction.COPY)
box.drag_source_add_uri_targets()
box.connect('drag-data-get', lambda _w, _c, data, *_: data.set_uris([Path(f).resolve().as_uri() for f in files]))
win.add(box)
win.move(int(x), int(y))
win.show_all()
print('listo', flush=True)
Gtk.main()
