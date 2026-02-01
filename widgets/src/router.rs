use ribir_core::prelude::{smallvec::smallvec, *};
use smallvec::SmallVec;

/// Declarative router that maps `Location` paths to UI widgets using pattern
/// matching.
///
/// ## Path Matching
///
/// Routes are evaluated in declaration order (first match wins) with three
/// segment types:
///
/// - **Static** - Exact path match (`/dashboard`)
/// - **Dynamic** - Colon-prefixed parameter capture (`/users/:id`)
/// - **Wildcard** - Asterisk suffix captures remaining path (`/files/*`), must
///   be final segment
///
/// ## Nested Routing
///
/// Create hierarchical layouts through:
/// 1. Parent route with wildcard termination (`/admin/*`)
/// 2. Child router with path continuation: ```text Parent: /admin/* Child:
///    /dashboard => /admin/dashboard ```
///
/// ## Examples
///
/// Basic parameter capture:
/// ```rust
/// use ribir::prelude::*;
///
/// router! {
///   @Route {
///     path: "/users/:id",
///     @fn_widget! {
///       let params = Provider::of::<RouterParams>(BuildCtx::get()).unwrap();
///       @Text { text: format!("User ID: {}", params.get_param("id").unwrap()) }
///     }
///   }
/// };
/// ```
///
/// Nested route structure:
/// ```rust
/// use ribir::prelude::*;
///
/// router! {
///   @Route {
///     path: "/admin/*",
///     @router! {
///       // Matches /admin/dashboard
///       @Route {
///         path: "/dashboard",
///         @text! { text: "Admin Dashboard" }
///       }
///       // Matches /admin/users/:id
///       @Route {
///         path: "/users/:id",
///         @text!{ text: "User Profile" }
///       }
///     }
///   }
/// };
/// ```
///
/// ## Widget Construction
///
/// The router uses lazy initialization for route widgets, requiring all routed
/// content to be [`GenWidget`] types. To reuse existing widget instances
/// instead of rebuilding on navigation, wrap content with [`ReuseId`] in
/// appropriate scopes.

#[declare]
pub struct Router {
  #[declare(default)]
  routes: Vec<Route>,
}

/// Captured path parameters from matched route segments.
///
/// Accessed through context providers in routed widgets:
///
/// ```no_run
/// use ribir::prelude::*;
///
/// let params = Provider::of::<RouterParams>(BuildCtx::get()).unwrap();
/// let user_id = params.get_param("id");
/// ```
#[derive(Default, Debug)]
pub struct RouterParams {
  params: SmallVec<[(String, String); 1]>,
}

/// Configuration for a single route mapping between path pattern and widget.
///
/// Path validation ensures:
/// - Wildcard (*) is only allowed as final segment
/// - Dynamic segments (:name) contain valid identifiers
/// - No reserved characters (:, *) in static segments
#[derive(Template)]
pub struct Route {
  #[template(field)]
  path: CowArc<str>,
  child: GenWidget,
}

impl RouterParams {
  /// Returns a read-only reference to the RouterParams provider of the context.
  pub fn of(ctx: &impl AsRef<ProviderCtx>) -> Option<QueryRef<'_, Self>> { Provider::of(ctx) }

  /// Returns captured parameter value if exists.
  ///
  /// Returns `None` if no parameter with the given name was matched.
  pub fn get_param(&self, name: &str) -> Option<&str> {
    self
      .params
      .iter()
      .find_map(|(k, v)| (k == name).then_some(v.as_str()))
  }
}

impl Router {
  /// Adds a new route to the router.
  pub fn add_route(&mut self, route: Route) { self.routes.push(route); }

  /// Matches registered routes against current path and returns active widget.
  ///
  /// - Uses location-aware context for path resolution
  /// - Injects matched parameters into widget context
  /// - Returns void widget when no routes match
  fn switch(&self, ctx: &BuildCtx) -> Widget<'static> {
    if let Some(route_params) = Provider::of::<RouterParams>(ctx)
      && let Some(path) = route_params.get_param("*")
    {
      return self.switch_to(path);
    }

    self.switch_to(Location::of(ctx).path())
  }

  /// Matches specific path against configured routes.
  fn switch_to(&self, path: &str) -> Widget<'static> {
    let mut params = smallvec![];

    let gen_widget = self.routes.iter().find_map(|route| {
      params.clear();
      route_match(path, &route.path, &mut params).then(|| route.child.clone())
    });

    gen_widget
      .map(move |gen_widget| {
        let params = RouterParams { params };
        providers! {
          providers: [Provider::new(params)],
          @{ gen_widget.gen_widget() }
        }
        .into_widget()
      })
      .unwrap_or_else(|| {
        log::warn!("No route found for: {}", path);
        Void::default().into_widget()
      })
  }
}

impl ComposeChild<'static> for Router {
  type Child = Vec<Route>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
    {
      let mut router = this.write();
      router
        .routes
        .extend(child.into_iter().filter(|route| {
          route
            .check_path()
            .inspect_err(|e| log::error!("Invalid route path '{}': {}", &route.path, e))
            .is_ok()
        }));

      router.forget_modifies();
    }

    let location = Location::state_of(BuildCtx::get());
    pipe! {
      let _ = $watcher(location);
      this.read().switch(BuildCtx::get())
    }
    .into_widget()
  }
}

impl Route {
  fn check_path(&self) -> Result<(), PathError> {
    let mut segs = split_path_segments(&self.path).peekable();
    while let Some(seg) = segs.next() {
      let seg = parse_segment(seg)?;
      if matches!(seg, PathSeg::Wildcard) && segs.peek().is_some() {
        return Err(PathError::WildcardNotLast);
      }
    }
    Ok(())
  }
}

fn route_match(path: &str, route_path: &str, params: &mut SmallVec<[(String, String); 1]>) -> bool {
  let mut path_iter = split_path_segments(path).peekable();
  let mut route_iter = split_path_segments(route_path).peekable();

  while let Some(route_seg) = route_iter.next() {
    let Ok(route_seg) = parse_segment(route_seg) else {
      return false;
    };
    match route_seg {
      PathSeg::Normal(expected) if path_iter.next() == Some(expected) => {}
      PathSeg::Dynamic(name) if path_iter.peek().is_some() => {
        let value = path_iter.next().unwrap();
        params.push((name.to_string(), value.to_string()));
      }
      PathSeg::Wildcard if route_iter.peek().is_none() => {
        params.push(("*".into(), path_iter.collect::<Vec<_>>().join("/")));
        return true;
      }
      _ => return false,
    }
  }

  // All route segments processed, check remaining path
  path_iter.peek().is_none()
}

/// Parse individual path segment with validation
///
/// # Rules
/// - `*` is wildcard (must be last segment)
/// - `:name` is dynamic parameter
/// - Static segments cannot contain `:` or `*`
///
/// # Examples
/// - Valid: `"users"`, `":id"`, `"*"`
/// - Invalid: `":"`, `"user*", `"::id"`
fn parse_segment(seg: &str) -> Result<PathSeg<'_>, PathError> {
  if seg == "*" {
    Ok(PathSeg::Wildcard)
  } else if let Some(name) = seg.strip_prefix(':') {
    if name.is_empty() {
      Err(PathError::EmptyDynamic)
    } else if name.contains(':') || name.contains('*') {
      Err(PathError::ReservedChar(name.to_string()))
    } else {
      Ok(PathSeg::Dynamic(name))
    }
  } else if seg.contains('*') || seg.contains(':') {
    Err(PathError::ReservedChar(seg.to_string()))
  } else {
    Ok(PathSeg::Normal(seg))
  }
}

fn split_path_segments(path: &str) -> impl Iterator<Item = &str> {
  path
    .trim_start_matches('/')
    .split('/')
    .filter(|s| !s.is_empty())
}

#[derive(Debug, PartialEq, Eq)]
enum PathSeg<'s> {
  Normal(&'s str),
  Dynamic(&'s str),
  Wildcard,
}

#[derive(Debug, thiserror::Error)]
enum PathError {
  #[error("Dynamic segment must have non-empty name")]
  EmptyDynamic,
  #[error("Segment contains reserved characters: {0}")]
  ReservedChar(String),
  #[error("Wildcard must be the final path segment")]
  WildcardNotLast,
}

#[cfg(test)]
mod tests {
  use ribir_core::{prelude::*, test_helper::*};

  use super::*;

  #[test]
  fn route_matcher() {
    let mut params = SmallVec::new();

    // Exact matches
    assert!(route_match("", "/", &mut params), "Root path should match");
    assert!(route_match("/a", "/a", &mut params), "Exact path match failed");

    // Dynamic parameters
    assert!(route_match("/user/42", "/user/:id", &mut params), "Dynamic segment failed");
    assert_eq!(params.as_slice(), &[("id".into(), "42".into())]);
    params.clear();

    // Wildcard matching
    assert!(route_match("/files/doc.txt", "/files/*", &mut params), "Wildcard match failed");
    assert_eq!(params[0].1, "doc.txt");
  }

  #[test]
  fn route_widgets() {
    reset_test_env!();

    const HOME_SIZE: Size = Size::new(100., 100.);
    const A_SIZE: Size = Size::new(200., 200.);
    const A_B_SIZE: Size = Size::new(230., 230.);
    const C_SIZE: Size = Size::new(300., 300.);
    const C_A_SIZE: Size = Size::new(320., 320.);

    let (nav, w_nav) = split_value("/");
    let wnd = TestWindow::from_widget(fn_widget! {
      let location = Location::state_of(BuildCtx::get());
      watch!($read(nav).to_string()).subscribe(move |v| {
        let _ = $write(location).resolve_relative(&v);
      });

      @Router {
        @Route {
          path: "/",
          @mock_box! { size: HOME_SIZE }
        }
        @Route {
          path: "/a",
          @mock_box! { size: A_SIZE }
        }
        @Route {
          path: "/a/b",
          @mock_box! { size: A_B_SIZE }
        }
        @Route {
          path: "/c/*",
          @router! {
            // nested
            @Route {
              path: "/",
              @mock_box! { size: C_SIZE }
            }
            @Route {
              path: "/a",
              @mock_box! { size: C_A_SIZE }
            }
          }
        }
        @Route {
          path: "/dyn/:id",
          @mock_box!{
            size: {
              let params = Provider::of::<RouterParams>(BuildCtx::get()).unwrap();
              match params.get_param("id") {
                Some("a") => A_SIZE,
                Some("c") => C_SIZE,
                _ => HOME_SIZE,
              }
            }
          }
        }
      }

    });

    wnd.draw_frame();
    wnd.assert_root_size(HOME_SIZE);

    *w_nav.write() = "/a";
    wnd.draw_frame();
    wnd.assert_root_size(A_SIZE);

    *w_nav.write() = "/a/b";
    wnd.draw_frame();
    wnd.assert_root_size(A_B_SIZE);

    *w_nav.write() = "/c";
    wnd.draw_frame();
    wnd.assert_root_size(C_SIZE);

    *w_nav.write() = "/c/a";
    wnd.draw_frame();
    wnd.assert_root_size(C_A_SIZE);

    *w_nav.write() = "/dyn/a";
    wnd.draw_frame();
    wnd.assert_root_size(A_SIZE);

    *w_nav.write() = "/dyn/c";
    wnd.draw_frame();
    wnd.assert_root_size(C_SIZE);

    *w_nav.write() = "/dyn";
    wnd.draw_frame();
    wnd.assert_root_size(ZERO_SIZE);
  }
}
