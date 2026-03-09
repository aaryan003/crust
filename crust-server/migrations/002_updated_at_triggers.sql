-- Updated At Triggers
-- VERSION: 1.0.0
-- Creates trigger function for automatic updated_at column updates

CREATE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply trigger to users
CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Apply trigger to organizations
CREATE TRIGGER update_organizations_updated_at
    BEFORE UPDATE ON organizations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Apply trigger to repositories
CREATE TRIGGER update_repositories_updated_at
    BEFORE UPDATE ON repositories
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Apply trigger to teams
CREATE TRIGGER update_teams_updated_at
    BEFORE UPDATE ON teams
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Apply trigger to pull_requests
CREATE TRIGGER update_pull_requests_updated_at
    BEFORE UPDATE ON pull_requests
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Apply trigger to pr_reviews
CREATE TRIGGER update_pr_reviews_updated_at
    BEFORE UPDATE ON pr_reviews
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Apply trigger to pr_comments
CREATE TRIGGER update_pr_comments_updated_at
    BEFORE UPDATE ON pr_comments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
