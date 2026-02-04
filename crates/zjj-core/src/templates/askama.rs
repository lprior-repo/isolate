    #[test]
    fn test_templates_have_cross_references() -> Result<(), TemplateError> {
        let workflow = render_template(TemplateType::Workflow, &valid_context()?)?;
        let beads = render_template(TemplateType::Beads, &valid_context()?)?;
        let _jujutsu = render_template(TemplateType::Jujutsu, &valid_context()?)?;

        // Verify cross-references between docs
        assert!(
            workflow.contains("bead") || workflow.contains("BEADS"),
            "Workflow template should reference beads documentation"
        );
        assert!(
            workflow.contains("jj") || workflow.contains("Jujutsu") || workflow.contains("JUJUTSU"),
            "Workflow template should reference Jujutsu documentation"
        );
        assert!(
            beads.contains("workflow") || beads.contains("WORKFLOW"),
            "Beads template should reference workflow documentation"
        );
        assert!(
            beads.contains("jj") || beads.contains("Jujutsu") || beads.contains("JUJUTSU"),
            "Beads template should reference Jujutsu documentation"
        );

        Ok(())
    }
