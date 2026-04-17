macro_rules! define_skill {
    // Entry point without `new` function
    (
        $( #[$meta:meta] )*
        $vis:vis struct $skill:ident: $trait:ident => $objects:ty[$object:ty] {
            $( $field_name:ident: $field_type:ty $( = $field_default:expr )?, )*
        }
    ) => {
        define_skill! {
            @$trait $objects[$object]
            extend_fields $trait
            fields { $( $field_name $field_type $( = $field_default )?, )* }
            struct { $( #[$meta] )* $vis $skill }
            new {
                setup {}
                args {}
                assigns {}
            }
        }
    };

    // Entry point with `new` function
    (
        $( #[$meta:meta] )*
        $vis:vis struct $skill:ident: $trait:ident => $objects:ty[$object:ty] {
            $( $field_name:ident: $field_type:ty $( = $field_default:expr )?, )*
        }

        $_new_vis:vis fn new( $( $arg_name:ident: $arg_type:ty ),* ) -> Self {
            $( $body:tt )*
        }
    ) => {
        define_skill! {
            @$trait $objects[$object]
            extend_fields $trait
            fields { $( $field_name $field_type |, )* }
            struct { $( #[$meta] )* $vis $skill }
            new {
                setup_body { $( $body )* }
                setup {}
                args { $( $arg_name $arg_type, )* }
            }
        }
    };

    // Entry point with `new` function + no_auto_impl flag
    (
        $( #[$meta:meta] )*
        $vis:vis struct $skill:ident: $trait:ident ( no_auto_impl ) => $objects:ty[$object:ty] {
            $( $field_name:ident: $field_type:ty $( = $field_default:expr )?, )*
        }

        $_new_vis:vis fn new( $( $arg_name:ident: $arg_type:ty ),* ) -> Self {
            $( $body:tt )*
        }
    ) => {
        define_skill! {
            @$trait skip_impl $objects[$object]
            extend_fields $trait
            fields { $( $field_name $field_type |, )* }
            struct { $( #[$meta] )* $vis $skill }
            new {
                setup_body { $( $body )* }
                setup {}
                args { $( $arg_name $arg_type, )* }
            }
        }
    };

    // Processing `new` function: Found the final `Self` return value
    (
        @$trait:ident skip_impl $objects:ty[$object:ty]
        extend_fields $extend_fields:ident
        fields { $( $fields:tt )* }
        struct { $( $struct:tt )* }
        new {
            setup_body {
                Self { $( $assign_name:ident: $assign_expr:expr, )* }
            }
            setup { $( $setup:tt )* }
            args { $( $args:tt )* }
        }
    ) => {
        define_skill! {
            @$trait skip_impl $objects[$object]
            extend_fields $trait
            fields { $( $fields )* }
            struct { $( $struct )* }
            new {
                setup { $( $setup )* }
                args { $( $args )* }
                assigns { $( $assign_name $assign_expr, )* }
            }
        }
    };

    (
        @$trait:ident $objects:ty[$object:ty]
        extend_fields $extend_fields:ident
        fields { $( $fields:tt )* }
        struct { $( $struct:tt )* }
        new {
            setup_body {
                Self { $( $assign_name:ident: $assign_expr:expr, )* } // <-
            }
            setup { $( $setup:tt )* }
            args { $( $args:tt )* }
        }
    ) => {
        define_skill! {
            @$trait $objects[$object]
            extend_fields $trait
            fields { $( $fields )* }
            struct { $( $struct )* }
            new {
                setup { $( $setup )* }
                args { $( $args )* }
                assigns { $( $assign_name $assign_expr, )* } // <-
            }
        }
    };

    // Processing `new` function: Pop next statement from body and continue
    (
        @$trait:ident skip_impl $objects:ty[$object:ty]
        extend_fields $extend_fields:ident
        fields { $( $fields:tt )* }
        struct { $( $struct:tt )* }
        new {
            setup_body {
                $stmt:stmt;
                $( $rest:tt )+
            }
            setup { $( $setup:tt )* }
            args { $( $args:tt )* }
        }
    ) => {
        define_skill! {
            @$trait skip_impl $objects[$object]
            extend_fields $trait
            fields { $( $fields )* }
            struct { $( $struct )* }
            new {
                setup_body { $( $rest )* }
                setup { $( $setup )* $stmt }
                args { $( $args )* }
            }
        }
    };

    (
        @$trait:ident $objects:ty[$object:ty]
        extend_fields $extend_fields:ident
        fields { $( $fields:tt )* }
        struct { $( $struct:tt )* }
        new {
            setup_body {
                $stmt:stmt; // <-
                $( $rest:tt )+
            }
            setup { $( $setup:tt )* }
            args { $( $args:tt )* }
        }
    ) => {
        define_skill! {
            @$trait $objects[$object]
            extend_fields $trait
            fields { $( $fields )* }
            struct { $( $struct )* }
            new {
                setup_body { $( $rest )* }   // <-
                setup { $( $setup )* $stmt } // <-
                args { $( $args )* }
            }
        }
    };

    // Extend `StrainDecaySkill`'s fields
    (
        @$trait:ident skip_impl $objects:ty[$object:ty]
        extend_fields StrainDecaySkill
        fields { $( $fields:tt )* }
        $( $rest:tt )*
    ) => {
        define_skill! {
            @$trait skip_impl $objects[$object]
            extend_fields StrainSkill
            fields {
                $( $fields )*
                strain_decay_skill_current_strain f64 = 0.0,
            }
            $( $rest )*
        }
    };

    (
        @$trait:ident $objects:ty[$object:ty]
        extend_fields StrainDecaySkill // <-
        fields { $( $fields:tt )* }
        $( $rest:tt )*
    ) => {
        define_skill! {
            @$trait $objects[$object]
            extend_fields StrainSkill
            fields {
                $( $fields )*
                strain_decay_skill_current_strain f64 = 0.0, // <-
            }
            $( $rest )*
        }
    };

    // Extend `HarmonicSkill`'s fields
    (
        @$trait:ident skip_impl $objects:ty[$object:ty]
        extend_fields HarmonicSkill
        fields { $( $fields:tt )* }
        $( $rest:tt )*
    ) => {
        define_skill! {
            @$trait skip_impl $objects[$object]
            fields {
                $( $fields )*
                harmonic_skill_object_difficulties Vec<f64> = Vec::with_capacity(256),
            }
            $( $rest )*
        }
    };

    (
        @$trait:ident $objects:ty[$object:ty]
        extend_fields HarmonicSkill // <-
        fields { $( $fields:tt )* }
        $( $rest:tt )*
    ) => {
        define_skill! {
            @$trait $objects[$object]
            fields {
                $( $fields )*
                harmonic_skill_object_difficulties Vec<f64> = Vec::with_capacity(256), // <-
            }
            $( $rest )*
        }
    };

    // Extend `StrainSkill`'s fields
    (
        @$trait:ident skip_impl $objects:ty[$object:ty]
        extend_fields StrainSkill
        fields { $( $fields:tt )* }
        $( $rest:tt )*
    ) => {
        define_skill! {
            @$trait skip_impl $objects[$object]
            fields {
                $( $fields )*
                strain_skill_current_section_peak f64 = 0.0,
                strain_skill_current_section_end f64 = 0.0,
                strain_skill_strain_peaks crate::util::strains_vec::StrainsVec
                    = crate::util::strains_vec::StrainsVec::with_capacity(256),
                strain_skill_object_strains Vec<f64> = Vec::with_capacity(256),
            }
            $( $rest )*
        }
    };

    (
        @$trait:ident $objects:ty[$object:ty]
        extend_fields StrainSkill // <-
        fields { $( $fields:tt )* }
        $( $rest:tt )*
    ) => {
        define_skill! {
            @$trait $objects[$object]
            fields {
                $( $fields )*
                strain_skill_current_section_peak f64 = 0.0, // <-
                strain_skill_current_section_end f64 = 0.0,  // <-
                strain_skill_strain_peaks crate::util::strains_vec::StrainsVec
                    = crate::util::strains_vec::StrainsVec::with_capacity(256), // <-
                // TODO: use `StrainsVec`?
                strain_skill_object_strains Vec<f64> = Vec::with_capacity(256), // <-
            }
            $( $rest )*
        }
    };

    // Extend `VariableLengthStrainSkill`'s fields
    (
        @$trait:ident skip_impl $objects:ty[$object:ty]
        extend_fields VariableLengthStrainSkill
        fields { $( $fields:tt )* }
        $( $rest:tt )*
    ) => {
        define_skill! {
            @$trait skip_impl $objects[$object]
            fields {
                $( $fields )*
                variable_length_strain_current_section_peak f64 = 0.0,
                variable_length_strain_current_section_begin f64 = 0.0,
                variable_length_strain_current_section_end f64 = 0.0,
                variable_length_strain_queued_strains Vec<(f64, f64)> = Vec::new(),
                variable_length_strain_total_length f64 = 0.0,
                variable_length_strain_peaks Vec<crate::any::difficulty::skills::StrainPeak> = Vec::with_capacity(64),
                variable_length_strain_object_strains Vec<f64> = Vec::with_capacity(256),
            }
            $( $rest )*
        }
    };

    (
        @$trait:ident $objects:ty[$object:ty]
        extend_fields VariableLengthStrainSkill // <-
        fields { $( $fields:tt )* }
        $( $rest:tt )*
    ) => {
        define_skill! {
            @$trait $objects[$object]
            fields {
                $( $fields )*
                variable_length_strain_current_section_peak f64 = 0.0, // <-
                variable_length_strain_current_section_begin f64 = 0.0,
                variable_length_strain_current_section_end f64 = 0.0,
                variable_length_strain_queued_strains Vec<(f64, f64)> = Vec::new(),
                variable_length_strain_total_length f64 = 0.0,
                variable_length_strain_peaks Vec<crate::any::difficulty::skills::StrainPeak> = Vec::with_capacity(64),
                variable_length_strain_object_strains Vec<f64> = Vec::with_capacity(256),
            }
            $( $rest )*
        }
    };

    // Parse field without default
    (
        @$trait:ident skip_impl $objects:ty[$object:ty]
        fields {
            $field_name:ident $field_type:ty,
            $( $fields:tt )*
        }
        struct { $( $struct:tt )* }
        new {
            setup { $( $setup:tt )* }
            args { $( $args:tt )* }
            assigns { $( $assigns:tt )* }
        }
    ) => {
        define_skill! {
            @$trait skip_impl $objects[$object]
            fields { $( $fields )* }
            struct { $( $struct )* $field_name $field_type, }
            new {
                setup { $( $setup )* }
                args { $( $args )* $field_name $field_type, }
                assigns { $( $assigns )* $field_name, }
            }
        }
    };

    (
        @$trait:ident $objects:ty[$object:ty]
        fields {
            $field_name:ident $field_type:ty, // <-
            $( $fields:tt )*
        }
        struct { $( $struct:tt )* }
        new {
            setup { $( $setup:tt )* }
            args { $( $args:tt )* }
            assigns { $( $assigns:tt )* }
        }
    ) => {
        define_skill! {
            @$trait $objects[$object]
            fields { $( $fields )* }
            struct { $( $struct )* $field_name $field_type, } // <-
            new {
                setup { $( $setup )* }
                args { $( $args )* $field_name $field_type, } // <-
                assigns { $( $assigns )* $field_name, }       // <-
            }
        }
    };

    // Parse field with default
    (
        @$trait:ident skip_impl $objects:ty[$object:ty]
        fields {
            $field_name:ident $field_type:ty = $field_default:expr,
            $( $fields:tt )*
        }
        struct { $( $struct:tt )* }
        new {
            setup { $( $setup:tt )* }
            args { $( $args:tt )* }
            assigns { $( $assigns:tt )* }
        }
    ) => {
        define_skill! {
            @$trait skip_impl $objects[$object]
            fields { $( $fields )* }
            struct { $( $struct )* $field_name $field_type, }
            new {
                setup { $( $setup )* }
                args { $( $args )* }
                assigns { $( $assigns )* $field_name $field_default, }
            }
        }
    };

    (
        @$trait:ident $objects:ty[$object:ty]
        fields {
            $field_name:ident $field_type:ty = $field_default:expr, // <-
            $( $fields:tt )*
        }
        struct { $( $struct:tt )* }
        new {
            setup { $( $setup:tt )* }
            args { $( $args:tt )* }
            assigns { $( $assigns:tt )* }
        }
    ) => {
        define_skill! {
            @$trait $objects[$object]
            fields { $( $fields )* }
            struct { $( $struct )* $field_name $field_type, } // <-
            new {
                setup { $( $setup )* }
                args { $( $args )* }
                assigns { $( $assigns )* $field_name $field_default, } // <-
            }
        }
    };

    // Parse field with but skip for `new` function
    (
        @$trait:ident skip_impl $objects:ty[$object:ty]
        fields {
            $field_name:ident $field_type:ty |,
            $( $fields:tt )*
        }
        struct { $( $struct:tt )* }
        $( $rest:tt )*
    ) => {
        define_skill! {
            @$trait skip_impl $objects[$object]
            fields { $( $fields )* }
            struct { $( $struct )* $field_name $field_type, }
            $( $rest )*
        }
    };

    (
        @$trait:ident $objects:ty[$object:ty]
        fields {
            $field_name:ident $field_type:ty |, // <-
            $( $fields:tt )*
        }
        struct { $( $struct:tt )* }
        $( $rest:tt )*
    ) => {
        define_skill! {
            @$trait $objects[$object]
            fields { $( $fields )* }
            struct { $( $struct )* $field_name $field_type, } // <-
            $( $rest )*
        }
    };

    // Final output
    (
        @$trait:ident skip_impl $objects:ty[$object:ty]
        fields {}
        struct {
            $( #[$meta:meta] )*
            $vis:vis $name:ident
            $( $field_name:ident $field_type:ty, )*
        }
        new {
            setup { $( $setup:stmt )* }
            args { $( $arg_name:ident $arg_type:ty, )* }
            assigns { $( $assign_name:ident $( $assign_expr:expr )?, )* }
        }
    ) => {
        $( #[$meta] )*
        $vis struct $name {
            $( $field_name: $field_type, )*
        }

        impl $name {
            $vis fn new(
                $( $arg_name: $arg_type, )*
            ) -> Self {
                $( $setup )*

                Self {
                    $( $assign_name $( : $assign_expr )?, )*
                }
            }
        }

    };

    (
        @$trait:ident $objects:ty[$object:ty]
        fields {}
        struct {
            $( #[$meta:meta] )*
            $vis:vis $name:ident
            $( $field_name:ident $field_type:ty, )*
        }
        new {
            setup { $( $setup:stmt )* }
            args { $( $arg_name:ident $arg_type:ty, )* }
            assigns { $( $assign_name:ident $( $assign_expr:expr )?, )* }
        }
    ) => {
        $( #[$meta] )*
        $vis struct $name {
            $( $field_name: $field_type, )*
        }

        impl $name {
            $vis fn new(
                $( $arg_name: $arg_type, )*
            ) -> Self {
                $( $setup )*

                Self {
                    $( $assign_name $( : $assign_expr )?, )*
                }
            }
        }

        const _: () = {
            #[expect(unused_imports, reason = "fine for macros")]
            use crate::{
                any::difficulty::{
                    object::{IDifficultyObject, IDifficultyObjects, HasStartTime},
                    skills::{StrainSkill, StrainDecaySkill, HarmonicSkill, VariableLengthStrainSkill},
                },
                util::strains_vec::StrainsVec,
            };

            define_skill!( @impl $trait $name $objects[$object] );
        };
    };

    // Implement `HarmonicSkill` trait
    ( @impl HarmonicSkill $name:ident $objects:ty[$object:ty] ) => {
        impl HarmonicSkill for $name {
            type DifficultyObject<'a> = $object;
            type DifficultyObjects<'a> = $objects;

            fn process<'a>(
                &mut self,
                curr: &Self::DifficultyObject<'a>,
                objects: &Self::DifficultyObjects<'a>,
            ) {
                let strain = self.strain_value_at(curr, objects);
                self.harmonic_skill_object_difficulties.push(strain);
            }

            fn cloned_difficulty_value(&self) -> f64 {
                Self::calculate_current_values(self).0
            }

            fn count_top_weighted_difficulties(&self, difficulty_value: f64) -> f64 {
                let (_, weight_sum) = Self::calculate_current_values(self);

                Self::count_top_weighted_object_difficulties(
                    &self.harmonic_skill_object_difficulties,
                    difficulty_value,
                    weight_sum,
                )
            }
        }
    };

    // Implement `StrainSkill` trait
    ( @impl StrainSkill $name:ident $objects:ty[$object:ty] ) => {
        impl StrainSkill for $name {
            type DifficultyObject<'a> = $object;
            type DifficultyObjects<'a> = $objects;

            fn process<'a>(
                &mut self,
                curr: &Self::DifficultyObject<'a>,
                objects: &Self::DifficultyObjects<'a>,
            ) {
                let section_length = f64::from(Self::SECTION_LENGTH);

                // * The first object doesn't generate a strain, so we begin with an incremented section end
                if curr.idx == 0 {
                    self.strain_skill_current_section_end =
                        f64::ceil(curr.start_time / section_length) * section_length;
                }

                while curr.start_time > self.strain_skill_current_section_end {
                    self.save_current_peak();
                    self.start_new_section_from(
                        self.strain_skill_current_section_end,
                        curr,
                        objects
                    );
                    self.strain_skill_current_section_end += section_length;
                }

                let strain = self.strain_value_at(curr, objects);
                self.strain_skill_current_section_peak
                    = f64::max(strain, self.strain_skill_current_section_peak);

                // * Store the strain value for the object
                self.strain_skill_object_strains.push(strain);
            }

            fn count_top_weighted_strains(&self, difficulty_value: f64) -> f64 {
                crate::any::difficulty::skills::count_top_weighted_strains(
                    &self.strain_skill_object_strains,
                    difficulty_value,
                    Self::DECAY_WEIGHT,
                )
            }

            fn save_current_peak(&mut self) {
                self.strain_skill_strain_peaks.push(self.strain_skill_current_section_peak);
            }

            fn start_new_section_from<'a>(
                &mut self,
                time: f64,
                curr: &Self::DifficultyObject<'a>,
                objects: &Self::DifficultyObjects<'a>,
            ) {
                self.strain_skill_current_section_peak
                    = self.calculate_initial_strain(time, curr, objects);
            }

            fn into_current_strain_peaks(self) -> StrainsVec {
                Self::get_current_strain_peaks(
                    self.strain_skill_strain_peaks,
                    self.strain_skill_current_section_peak,
                )
            }

            fn difficulty_value(current_strain_peaks: StrainsVec) -> f64 {
                crate::any::difficulty::skills::difficulty_value(
                    current_strain_peaks,
                    Self::DECAY_WEIGHT,
                )
            }

            fn into_difficulty_value(self) -> f64 {
                Self::difficulty_value(
                    Self::get_current_strain_peaks(
                        self.strain_skill_strain_peaks,
                        self.strain_skill_current_section_peak,
                    )
                )
            }

            fn cloned_difficulty_value(&self) -> f64 {
                Self::difficulty_value(
                    Self::get_current_strain_peaks(
                        self.strain_skill_strain_peaks.clone(),
                        self.strain_skill_current_section_peak,
                    )
                )
            }
        }
    };

    // Implement `StrainDecaySkill` and `StrainSkill` traits
    ( @impl StrainDecaySkill $name:ident $objects:ty[$object:ty] ) => {
        define_skill!( @impl StrainSkill $name $objects[$object] );

        impl StrainDecaySkill for $name {
            fn calculate_initial_strain<'a>(
                &self,
                time: f64,
                curr: &Self::DifficultyObject<'a>,
                objects: &Self::DifficultyObjects<'a>,
            ) -> f64 {
                let prev_start_time = curr
                    .previous(0, objects)
                    .map_or(0.0, HasStartTime::start_time);

                self.strain_decay_skill_current_strain
                    * Self::strain_decay(time - prev_start_time)
            }

            fn strain_value_at<'a>(
                &mut self,
                curr: &Self::DifficultyObject<'a>,
                objects: &Self::DifficultyObjects<'a>,
            ) -> f64 {
                self.strain_decay_skill_current_strain
                    *= Self::strain_decay(curr.delta_time);
                self.strain_decay_skill_current_strain
                    += self.strain_value_of(curr, objects) * Self::SKILL_MULTIPLIER;

                self.strain_decay_skill_current_strain
            }

            fn strain_decay(ms: f64) -> f64 {
                crate::any::difficulty::skills::strain_decay(ms, Self::STRAIN_DECAY_BASE)
            }
        }
    };

    // Implement `VariableLengthStrainSkill` trait
    ( @impl VariableLengthStrainSkill $name:ident $objects:ty[$object:ty] ) => {
        impl VariableLengthStrainSkill for $name {
            type DifficultyObject<'a> = $object;
            type DifficultyObjects<'a> = $objects;

            fn process<'a>(
                &mut self,
                curr: &Self::DifficultyObject<'a>,
                objects: &Self::DifficultyObjects<'a>,
            ) {
                let section_length = f64::from(Self::MAX_SECTION_LENGTH);

                if curr.idx == 0 {
                    self.variable_length_strain_current_section_begin = curr.start_time;
                    self.variable_length_strain_current_section_end =
                        self.variable_length_strain_current_section_begin + section_length;
                    self.variable_length_strain_current_section_peak = self.strain_value_at(curr, objects);
                    self.variable_length_strain_object_strains.push(self.variable_length_strain_current_section_peak);
                    return;
                }

                while curr.start_time > self.variable_length_strain_current_section_end {
                    self.save_current_peak(
                        self.variable_length_strain_current_section_end - self.variable_length_strain_current_section_begin,
                    );
                    self.variable_length_strain_current_section_begin = self.variable_length_strain_current_section_end;

                    if !self.variable_length_strain_queued_strains.is_empty() {
                        let (strain, start_time) = self.variable_length_strain_queued_strains.remove(0);
                        self.variable_length_strain_current_section_end = start_time + section_length;
                        self.start_new_section_from(
                            self.variable_length_strain_current_section_begin,
                            curr,
                            objects,
                        );
                        self.variable_length_strain_current_section_peak =
                            f64::max(self.variable_length_strain_current_section_peak, strain);
                    } else {
                        self.variable_length_strain_current_section_end =
                            self.variable_length_strain_current_section_begin + section_length;
                        self.start_new_section_from(
                            self.variable_length_strain_current_section_begin,
                            curr,
                            objects,
                        );
                    }
                }

                let current_strain = self.strain_value_at(curr, objects);
                self.variable_length_strain_object_strains.push(current_strain);

                if current_strain > self.variable_length_strain_current_section_peak {
                    self.variable_length_strain_queued_strains.clear();
                    self.save_current_peak(
                        curr.start_time - self.variable_length_strain_current_section_begin,
                    );
                    self.variable_length_strain_current_section_begin = curr.start_time;
                    self.variable_length_strain_current_section_end =
                        self.variable_length_strain_current_section_begin + section_length;
                    self.variable_length_strain_current_section_peak = current_strain;
                } else {
                    while let Some(&(strain, _)) = self.variable_length_strain_queued_strains.last() {
                        if strain < current_strain {
                            self.variable_length_strain_queued_strains.pop();
                        } else {
                            break;
                        }
                    }
                    self.variable_length_strain_queued_strains.push((current_strain, curr.start_time));
                }
            }

            fn save_current_peak(&mut self, section_length: f64) {
                let peak = crate::any::difficulty::skills::StrainPeak {
                    value: self.variable_length_strain_current_section_peak,
                    section_length: f64::round(section_length),
                };

                let pos = self.variable_length_strain_peaks
                    .partition_point(|p| p.value <= peak.value);
                self.variable_length_strain_peaks.insert(pos, peak);
                self.variable_length_strain_total_length += section_length;

                let max_stored_sections = 11.0 / (1.0 - Self::DECAY_WEIGHT);
                while self.variable_length_strain_total_length > max_stored_sections * f64::from(Self::MAX_SECTION_LENGTH) {
                    self.variable_length_strain_total_length -= self.variable_length_strain_peaks[0].section_length;
                    self.variable_length_strain_peaks.remove(0);
                }
            }

            fn start_new_section_from<'a>(
                &mut self,
                time: f64,
                curr: &Self::DifficultyObject<'a>,
                objects: &Self::DifficultyObjects<'a>,
            ) {
                self.variable_length_strain_current_section_peak = self.calculate_initial_strain(time, curr, objects);
            }

            fn get_current_strain_peaks(&self) -> impl Iterator<Item = crate::any::difficulty::skills::StrainPeak> {
                let current_peak = crate::any::difficulty::skills::StrainPeak {
                    value: self.variable_length_strain_current_section_peak,
                    section_length: self.variable_length_strain_current_section_end
                        - self.variable_length_strain_current_section_begin,
                };
                let mut p = self.variable_length_strain_peaks.clone();
                p.push(current_peak);
                p.into_iter()
            }

            fn cloned_difficulty_value(&self) -> f64 {
                let peaks: Vec<_> = self.get_current_strain_peaks().collect();
                let mut strains = crate::any::difficulty::skills::get_reduced_strain_peaks(
                    peaks,
                    $name::REDUCED_SECTION_TIME,
                    $name::REDUCED_STRAIN_BASELINE,
                );

                strains.retain(|p| p.value > 0.0);
                strains.sort_unstable_by(|a, b| b.value.total_cmp(&a.value));

                let max_section_length = f64::from(Self::MAX_SECTION_LENGTH);

                let mut difficulty = 0.0;
                let mut time = 0.0;

                // * Difficulty is a continuous weighted sum of the sorted strains
                for strain in &strains {
                    // * Weighting function can be thought of as:
                    // *         b
                    // *         ∫ DecayWeight^x dx
                    // *         a
                    // *     where a = startTime and b = endTime
                    // *
                    // *     Technically, the function below has been slightly modified from the equation above.
                    // *     The real function would be
                    // *         double weight = Math.Pow(DecayWeight, startTime) - Math.Pow(DecayWeight, endTime);
                    // *         ...
                    // *         return difficulty / Math.Log(1 / DecayWeight);
                    // *     E.g. for a DecayWeight of 0.9, we're multiplying by 10 instead of 9.49122...
                    // *
                    // *     This change makes it so that a map composed solely of MaxSectionLength chunks will have the exact same value when summed in this class and StrainSkill.
                    // *     Doing this ensures the relationship between strain values and difficulty values remains the same between the two classes.
                    let start_time = time;
                    let end_time = time + strain.section_length / max_section_length;

                    let weight = f64::powf(Self::DECAY_WEIGHT, start_time)
                        - f64::powf(Self::DECAY_WEIGHT, end_time);

                    difficulty += strain.value * weight;
                    time = end_time;
                }

                difficulty / (1.0 - Self::DECAY_WEIGHT)
            }

            fn count_top_weighted_strains(&self, difficulty_value: f64) -> f64 {
                crate::any::difficulty::skills::count_top_weighted_strains(
                    &self.variable_length_strain_object_strains,
                    difficulty_value,
                    Self::DECAY_WEIGHT,
                )
            }

            fn into_current_strain_peaks(mut self) -> Vec<f64> {
                let current_peak = crate::any::difficulty::skills::StrainPeak {
                    value: self.variable_length_strain_current_section_peak,
                    section_length: self.variable_length_strain_current_section_end
                        - self.variable_length_strain_current_section_begin,
                };
                self.variable_length_strain_peaks.push(current_peak);
                self.variable_length_strain_peaks
                    .into_iter()
                    .map(|p| p.value)
                    .collect()
            }
        }
    };
}
