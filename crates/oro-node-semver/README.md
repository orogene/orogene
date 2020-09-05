# Oro Node Semver


TODO:

https://github.com/dart-lang/pub_semver/blob/master/lib/src/version_range.dart cotains a good impl of below operations needed for PubGrub


* bool allowsAll(VersionConstraint other);
  /// Returns `true` if this constraint allows all the versions that [other]
  /// allows.
  e.g. foo.allowsAll(other) // foo ^1.5.0 is a subset of foo ^1.0.0


* bool allowsAny(VersionConstraint other);
  /// Returns `true` if this constraint allows any of the versions that [other]
  /// allows.
  e.g. !foo.allowsAny(other) // foo ^2.0.0 is disjoint with foo ^1.0.0

* VersionConstraint difference(VersionConstraint other);
  /// Returns a [VersionConstraint] that allows [Version]s allowed by this but
  /// not [other].
  e.g. positive.constraint.difference(negative.constraint) // foo ^1.0.0 ∩ not foo ^1.5.0 → foo >=1.0.0 <1.5.0

* VersionConstraint intersect(VersionConstraint other);
  /// Returns a [VersionConstraint] that only allows [Version]s allowed by both
  /// this and [other].
  e.g. constraint.intersect(other.constraint); // foo ^1.0.0 ∩ foo >=1.5.0 <3.0.0 → foo ^1.5.0

* VersionConstraint union(VersionConstraint other);
  /// Returns a [VersionConstraint] that allows [Version]s allowed by either
  /// this or [other].
  e.g. constraint.union(other.constraint); // not foo ^1.0.0 ∩ not foo >=1.5.0 <3.0.0 → not foo >=1.0.0 <3.0.0
