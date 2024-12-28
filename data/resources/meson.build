gnome = import('gnome')
gnome.post_install(
  glib_compile_schemas: true,
  gtk_update_icon_cache: true,
  #update_desktop_database: true,
)

dependency('libadwaita-1')
dependency('gtk4')

# Resources
blueprints = custom_target('blueprints',
  input: files(
    'ui/shortcuts.blp',
    'ui/window.blp',
  ),
  output: '.',
  command: [find_program('blueprint-compiler'), 'batch-compile', '@OUTPUT@', '@CURRENT_SOURCE_DIR@', '@INPUT@'],
)

resources = gnome.compile_resources(
  'resources',
  'resources.gresource.xml',
  gresource_bundle: true,
  source_dir: meson.current_build_dir(),
  install: true,
  dependencies: blueprints,
  install_dir: pkgdatadir,
)
