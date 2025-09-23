use crate::prelude::*;
// Infrastructure State Model
// Model for representing infrastructure component states

use crate::{OpError, OpResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Unique identifier for infrastructure components
pub type ComponentId = String;

/// Version identifier for infrastructure components
pub type ComponentVersion = String;

/// Infrastructure component types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentType {
    /// Virtual machine or compute instance
    VirtualMachine,
    /// Container instance
    Container,
    /// Load balancer
    LoadBalancer,
    /// Database instance
    Database,
    /// Network component (VPC, subnet, security group)
    Network,
    /// Storage component (volume, bucket)
    Storage,
    /// Application service
    Service,
    /// Configuration or secret
    Configuration,
    /// DNS record
    DNS,
    /// Certificate
    Certificate,
    /// Custom component type
    Custom(String),
}

impl fmt::Display for ComponentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComponentType::VirtualMachine => write!(f, "virtual-machine"),
            ComponentType::Container => write!(f, "container"),
            ComponentType::LoadBalancer => write!(f, "load-balancer"),
            ComponentType::Database => write!(f, "database"),
            ComponentType::Network => write!(f, "network"),
            ComponentType::Storage => write!(f, "storage"),
            ComponentType::Service => write!(f, "service"),
            ComponentType::Configuration => write!(f, "configuration"),
            ComponentType::DNS => write!(f, "dns"),
            ComponentType::Certificate => write!(f, "certificate"),
            ComponentType::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

/// Infrastructure component health status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Component is healthy and functioning normally
    Healthy,
    /// Component is degraded but still functional
    Degraded,
    /// Component is unhealthy and may not be functional
    Unhealthy,
    /// Component health status is unknown
    Unknown,
}

impl HealthStatus {
    /// Check if status indicates the component is operational
    pub fn is_operational(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::Degraded)
    }

    /// Check if status requires attention
    pub fn needs_attention(&self) -> bool {
        matches!(self, HealthStatus::Degraded | HealthStatus::Unhealthy | HealthStatus::Unknown)
    }
}

/// Infrastructure component deployment status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeploymentStatus {
    /// Component is being planned/designed
    Planning,
    /// Component is being deployed
    Deploying,
    /// Component is successfully deployed and active
    Deployed,
    /// Component is being updated/modified
    Updating,
    /// Component is being destroyed/removed
    Destroying,
    /// Component has been destroyed
    Destroyed,
    /// Component deployment failed
    Failed,
    /// Component is in an unknown state
    Unknown,
}

impl DeploymentStatus {
    /// Check if status is terminal (no further changes expected)
    pub fn is_terminal(&self) -> bool {
        matches!(self, DeploymentStatus::Destroyed | DeploymentStatus::Failed)
    }

    /// Check if status indicates active deployment
    pub fn is_active(&self) -> bool {
        matches!(self, DeploymentStatus::Deployed)
    }

    /// Check if status indicates transitional state
    pub fn is_transitional(&self) -> bool {
        matches!(
            self, 
            DeploymentStatus::Planning | 
            DeploymentStatus::Deploying | 
            DeploymentStatus::Updating | 
            DeploymentStatus::Destroying
        )
    }
}

/// Infrastructure component configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConfiguration {
    /// Configuration parameters
    pub parameters: HashMap<String, String>,
    /// Configuration schema version
    pub schema_version: String,
    /// Configuration checksum for change detection
    pub checksum: Option<String>,
}

impl ComponentConfiguration {
    /// Create new component configuration
    pub fn new(schema_version: String) -> Self {
        Self {
            parameters: HashMap::new(),
            schema_version,
            checksum: None,
        }
    }

    /// Add configuration parameter
    pub fn with_parameter(mut self, key: String, value: String) -> Self {
        self.parameters.insert(key, value);
        self
    }

    /// Set configuration checksum
    pub fn with_checksum(mut self, checksum: String) -> Self {
        self.checksum = Some(checksum);
        self
    }

    /// Get parameter value
    pub fn get_parameter(&self, key: &str) -> Option<&String> {
        self.parameters.get(key)
    }

    /// Check if configuration has changed (different checksum)
    pub fn has_changed(&self, other_checksum: Option<&str>) -> bool {
        match (&self.checksum, other_checksum) {
            (Some(current), Some(other)) => current != other,
            (None, None) => false,
            _ => true, // If one has checksum and other doesn't, consider changed
        }
    }
}

impl Default for ComponentConfiguration {
    fn default() -> Self {
        Self::new("1.0".to_string())
    }
}

/// Infrastructure component resource information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentResources {
    /// CPU allocation/usage
    pub cpu: Option<String>,
    /// Memory allocation/usage
    pub memory: Option<String>,
    /// Storage allocation/usage
    pub storage: Option<String>,
    /// Network bandwidth allocation/usage
    pub network: Option<String>,
    /// Custom resource metrics
    pub custom: HashMap<String, String>,
}

impl ComponentResources {
    /// Create new component resources
    pub fn new() -> Self {
        Self {
            cpu: None,
            memory: None,
            storage: None,
            network: None,
            custom: HashMap::new(),
        }
    }

    /// Set CPU resource
    pub fn with_cpu(mut self, cpu: String) -> Self {
        self.cpu = Some(cpu);
        self
    }

    /// Set memory resource
    pub fn with_memory(mut self, memory: String) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Set storage resource
    pub fn with_storage(mut self, storage: String) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Add custom resource metric
    pub fn with_custom_resource(mut self, key: String, value: String) -> Self {
        self.custom.insert(key, value);
        self
    }
}

impl Default for ComponentResources {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete infrastructure component state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfrastructureComponentState {
    /// Unique component identifier
    pub component_id: ComponentId,
    /// Human-readable component name
    pub name: String,
    /// Component type classification
    pub component_type: ComponentType,
    /// Current deployment status
    pub deployment_status: DeploymentStatus,
    /// Current health status
    pub health_status: HealthStatus,
    /// Component version
    pub version: ComponentVersion,
    /// Component configuration
    pub configuration: ComponentConfiguration,
    /// Resource allocation/usage
    pub resources: ComponentResources,
    /// Component dependencies (other components this depends on)
    pub dependencies: HashSet<ComponentId>,
    /// Components that depend on this one
    pub dependents: HashSet<ComponentId>,
    /// Geographic region/zone where component is deployed
    pub region: Option<String>,
    /// Availability zone within region
    pub availability_zone: Option<String>,
    /// Component tags/labels for organization
    pub tags: HashMap<String, String>,
    /// Component creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Last health check timestamp
    pub last_health_check: Option<chrono::DateTime<chrono::Utc>>,
    /// Error message if component is in error state
    pub error_message: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl InfrastructureComponentState {
    /// Create new infrastructure component state
    pub fn new(
        component_id: ComponentId,
        name: String,
        component_type: ComponentType,
    ) -> Self {
        let now = chrono::Utc::now();
        
        Self {
            component_id,
            name,
            component_type,
            deployment_status: DeploymentStatus::Planning,
            health_status: HealthStatus::Unknown,
            version: "1.0.0".to_string(),
            configuration: ComponentConfiguration::default(),
            resources: ComponentResources::default(),
            dependencies: HashSet::new(),
            dependents: HashSet::new(),
            region: None,
            availability_zone: None,
            tags: HashMap::new(),
            created_at: now,
            updated_at: now,
            last_health_check: None,
            error_message: None,
            metadata: HashMap::new(),
        }
    }

    /// Set component version
    pub fn with_version(mut self, version: ComponentVersion) -> Self {
        self.version = version;
        self
    }

    /// Set component region
    pub fn with_region(mut self, region: String) -> Self {
        self.region = Some(region);
        self
    }

    /// Set availability zone
    pub fn with_availability_zone(mut self, az: String) -> Self {
        self.availability_zone = Some(az);
        self
    }

    /// Add component tag
    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }

    /// Add dependency on another component
    pub fn add_dependency(&mut self, dependency_id: ComponentId) {
        self.dependencies.insert(dependency_id);
        self.updated_at = chrono::Utc::now();
    }

    /// Remove dependency on another component
    pub fn remove_dependency(&mut self, dependency_id: &ComponentId) {
        self.dependencies.remove(dependency_id);
        self.updated_at = chrono::Utc::now();
    }

    /// Add dependent component
    pub fn add_dependent(&mut self, dependent_id: ComponentId) {
        self.dependents.insert(dependent_id);
        self.updated_at = chrono::Utc::now();
    }

    /// Remove dependent component
    pub fn remove_dependent(&mut self, dependent_id: &ComponentId) {
        self.dependents.remove(dependent_id);
        self.updated_at = chrono::Utc::now();
    }

    /// Update deployment status
    pub fn update_deployment_status(&mut self, status: DeploymentStatus) {
        self.deployment_status = status;
        self.updated_at = chrono::Utc::now();
    }

    /// Update health status
    pub fn update_health_status(&mut self, status: HealthStatus) {
        self.health_status = status;
        self.last_health_check = Some(chrono::Utc::now());
        self.updated_at = chrono::Utc::now();
    }

    /// Set error message
    pub fn set_error(&mut self, error_message: String) {
        self.error_message = Some(error_message);
        self.updated_at = chrono::Utc::now();
    }

    /// Clear error message
    pub fn clear_error(&mut self) {
        self.error_message = None;
        self.updated_at = chrono::Utc::now();
    }

    /// Add metadata entry
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.updated_at = chrono::Utc::now();
    }

    /// Check if component is ready for deployment
    pub fn is_ready_for_deployment(&self) -> bool {
        matches!(self.deployment_status, DeploymentStatus::Planning) &&
        !self.dependencies.is_empty() || self.dependencies.is_empty()
    }

    /// Check if component can be safely destroyed
    pub fn can_be_destroyed(&self) -> bool {
        self.dependents.is_empty() && 
        matches!(self.deployment_status, DeploymentStatus::Deployed | DeploymentStatus::Failed)
    }

    /// Get component age (time since creation)
    pub fn age(&self) -> chrono::Duration {
        chrono::Utc::now() - self.created_at
    }

    /// Get time since last update
    pub fn time_since_update(&self) -> chrono::Duration {
        chrono::Utc::now() - self.updated_at
    }

    /// Check if health status is stale (no recent health check)
    pub fn is_health_stale(&self, threshold: chrono::Duration) -> bool {
        match self.last_health_check {
            Some(last_check) => (chrono::Utc::now() - last_check) > threshold,
            None => true,
        }
    }
}

/// Infrastructure topology representing relationships between components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfrastructureTopology {
    /// All components in the infrastructure
    pub components: HashMap<ComponentId, InfrastructureComponentState>,
    /// Topology metadata
    pub metadata: HashMap<String, String>,
    /// Topology creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last topology update
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl InfrastructureTopology {
    /// Create new infrastructure topology
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        
        Self {
            components: HashMap::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add component to topology
    pub fn add_component(&mut self, component: InfrastructureComponentState) {
        self.components.insert(component.component_id.clone(), component);
        self.updated_at = chrono::Utc::now();
    }

    /// Remove component from topology
    pub fn remove_component(&mut self, component_id: &ComponentId) -> Option<InfrastructureComponentState> {
        let removed = self.components.remove(component_id);
        
        if removed.is_some() {
            // Remove this component from all dependency relationships
            for component in self.components.values_mut() {
                component.remove_dependency(component_id);
                component.remove_dependent(component_id);
            }
            self.updated_at = chrono::Utc::now();
        }
        
        removed
    }

    /// Get component by ID
    pub fn get_component(&self, component_id: &ComponentId) -> Option<&InfrastructureComponentState> {
        self.components.get(component_id)
    }

    /// Get mutable component by ID
    pub fn get_component_mut(&mut self, component_id: &ComponentId) -> Option<&mut InfrastructureComponentState> {
        self.components.get_mut(component_id)
    }

    /// Create dependency relationship between components
    pub fn add_dependency(&mut self, dependent_id: &ComponentId, dependency_id: &ComponentId) -> OpResult<()> {
        // Check that both components exist
        if !self.components.contains_key(dependent_id) {
            return Err(OpError::ExecutionFailed(
                format!("Dependent component not found: {}", dependent_id)
            ));
        }
        
        if !self.components.contains_key(dependency_id) {
            return Err(OpError::ExecutionFailed(
                format!("Dependency component not found: {}", dependency_id)
            ));
        }

        // Check for circular dependencies
        if self.would_create_cycle(dependent_id, dependency_id) {
            return Err(OpError::ExecutionFailed(
                "Adding dependency would create circular dependency".to_string()
            ));
        }

        // Add dependency relationship
        if let Some(dependent) = self.components.get_mut(dependent_id) {
            dependent.add_dependency(dependency_id.clone());
        }

        if let Some(dependency) = self.components.get_mut(dependency_id) {
            dependency.add_dependent(dependent_id.clone());
        }

        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    /// Remove dependency relationship between components
    pub fn remove_dependency(&mut self, dependent_id: &ComponentId, dependency_id: &ComponentId) {
        if let Some(dependent) = self.components.get_mut(dependent_id) {
            dependent.remove_dependency(dependency_id);
        }

        if let Some(dependency) = self.components.get_mut(dependency_id) {
            dependency.remove_dependent(dependent_id);
        }

        self.updated_at = chrono::Utc::now();
    }

    /// Check if adding a dependency would create a circular dependency
    pub fn would_create_cycle(&self, from: &ComponentId, to: &ComponentId) -> bool {
        if from == to {
            return true;
        }

        let mut visited = HashSet::new();
        self.has_path_to(to, from, &mut visited)
    }

    /// Helper function to check if there's a path from one component to another
    fn has_path_to(&self, from: &ComponentId, to: &ComponentId, visited: &mut HashSet<ComponentId>) -> bool {
        if visited.contains(from) {
            return false; // Avoid infinite loops
        }
        
        visited.insert(from.clone());

        if let Some(component) = self.components.get(from) {
            for dependency in &component.dependencies {
                if dependency == to || self.has_path_to(dependency, to, visited) {
                    return true;
                }
            }
        }

        false
    }

    /// Get all components by type
    pub fn get_components_by_type(&self, component_type: &ComponentType) -> Vec<&InfrastructureComponentState> {
        self.components
            .values()
            .filter(|comp| comp.component_type == *component_type)
            .collect()
    }

    /// Get all components by deployment status
    pub fn get_components_by_deployment_status(&self, status: &DeploymentStatus) -> Vec<&InfrastructureComponentState> {
        self.components
            .values()
            .filter(|comp| comp.deployment_status == *status)
            .collect()
    }

    /// Get all components by health status
    pub fn get_components_by_health_status(&self, status: &HealthStatus) -> Vec<&InfrastructureComponentState> {
        self.components
            .values()
            .filter(|comp| comp.health_status == *status)
            .collect()
    }

    /// Get components with no dependencies (leaf nodes for deployment)
    pub fn get_deployment_ready_components(&self) -> Vec<&InfrastructureComponentState> {
        self.components
            .values()
            .filter(|comp| comp.is_ready_for_deployment())
            .collect()
    }

    /// Get components that can be safely destroyed
    pub fn get_destroyable_components(&self) -> Vec<&InfrastructureComponentState> {
        self.components
            .values()
            .filter(|comp| comp.can_be_destroyed())
            .collect()
    }

    /// Get topological sort order for deployment
    pub fn get_deployment_order(&self) -> Result<Vec<ComponentId>, OpError> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        for component_id in self.components.keys() {
            if !visited.contains(component_id) {
                self.topological_sort_visit(component_id, &mut visited, &mut visiting, &mut result)?;
            }
        }

        // No need to reverse - components are already in dependency order
        Ok(result)
    }

    /// Helper for topological sort with cycle detection
    fn topological_sort_visit(
        &self,
        component_id: &ComponentId,
        visited: &mut HashSet<ComponentId>,
        visiting: &mut HashSet<ComponentId>,
        result: &mut Vec<ComponentId>,
    ) -> OpResult<()> {
        if visiting.contains(component_id) {
            return Err(OpError::ExecutionFailed(
                "Circular dependency detected in topology".to_string()
            ));
        }

        if visited.contains(component_id) {
            return Ok(());
        }

        visiting.insert(component_id.clone());

        if let Some(component) = self.components.get(component_id) {
            for dependency in &component.dependencies {
                self.topological_sort_visit(dependency, visited, visiting, result)?;
            }
        }

        visiting.remove(component_id);
        visited.insert(component_id.clone());
        result.push(component_id.clone());

        Ok(())
    }

    /// Get infrastructure statistics
    pub fn get_statistics(&self) -> InfrastructureStatistics {
        let mut stats = InfrastructureStatistics::default();
        
        stats.total_components = self.components.len();

        for component in self.components.values() {
            // Count by deployment status
            match component.deployment_status {
                DeploymentStatus::Planning => stats.planning_components += 1,
                DeploymentStatus::Deploying => stats.deploying_components += 1,
                DeploymentStatus::Deployed => stats.deployed_components += 1,
                DeploymentStatus::Updating => stats.updating_components += 1,
                DeploymentStatus::Destroying => stats.destroying_components += 1,
                DeploymentStatus::Destroyed => stats.destroyed_components += 1,
                DeploymentStatus::Failed => stats.failed_components += 1,
                DeploymentStatus::Unknown => stats.unknown_deployment_components += 1,
            }

            // Count by health status
            match component.health_status {
                HealthStatus::Healthy => stats.healthy_components += 1,
                HealthStatus::Degraded => stats.degraded_components += 1,
                HealthStatus::Unhealthy => stats.unhealthy_components += 1,
                HealthStatus::Unknown => stats.unknown_health_components += 1,
            }

            // Count by component type
            let type_name = component.component_type.to_string();
            *stats.components_by_type.entry(type_name).or_insert(0) += 1;
        }

        stats
    }

    /// Get component count
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Check if topology is healthy (all components operational)
    pub fn is_healthy(&self) -> bool {
        self.components
            .values()
            .all(|comp| comp.health_status.is_operational())
    }

    /// Find components with stale health status
    pub fn find_stale_health_components(&self, threshold: chrono::Duration) -> Vec<&InfrastructureComponentState> {
        self.components
            .values()
            .filter(|comp| comp.is_health_stale(threshold))
            .collect()
    }
}

impl Default for InfrastructureTopology {
    fn default() -> Self {
        Self::new()
    }
}

/// Infrastructure statistics summary
#[derive(Debug, Clone, Default)]
pub struct InfrastructureStatistics {
    pub total_components: usize,
    pub planning_components: usize,
    pub deploying_components: usize,
    pub deployed_components: usize,
    pub updating_components: usize,
    pub destroying_components: usize,
    pub destroyed_components: usize,
    pub failed_components: usize,
    pub unknown_deployment_components: usize,
    pub healthy_components: usize,
    pub degraded_components: usize,
    pub unhealthy_components: usize,
    pub unknown_health_components: usize,
    pub components_by_type: HashMap<String, usize>,
}

impl InfrastructureStatistics {
    /// Calculate deployment success rate
    pub fn deployment_success_rate(&self) -> f64 {
        if self.total_components == 0 {
            0.0
        } else {
            (self.deployed_components as f64 / self.total_components as f64) * 100.0
        }
    }

    /// Calculate health success rate
    pub fn health_success_rate(&self) -> f64 {
        if self.total_components == 0 {
            0.0
        } else {
            (self.healthy_components as f64 / self.total_components as f64) * 100.0
        }
    }

    /// Get operational components (healthy + degraded)
    pub fn operational_components(&self) -> usize {
        self.healthy_components + self.degraded_components
    }

    /// Get problematic components (unhealthy + failed)
    pub fn problematic_components(&self) -> usize {
        self.unhealthy_components + self.failed_components
    }

    /// Get active components (deployed + updating)
    pub fn active_components(&self) -> usize {
        self.deployed_components + self.updating_components
    }

    /// Get transitional components (deploying + updating + destroying)
    pub fn transitional_components(&self) -> usize {
        self.deploying_components + self.updating_components + self.destroying_components
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_type_display() {
        assert_eq!(ComponentType::VirtualMachine.to_string(), "virtual-machine");
        assert_eq!(ComponentType::LoadBalancer.to_string(), "load-balancer");
        assert_eq!(ComponentType::Custom("firewall".to_string()).to_string(), "custom:firewall");
    }

    #[test]
    fn test_health_status_checks() {
        assert!(HealthStatus::Healthy.is_operational());
        assert!(HealthStatus::Degraded.is_operational());
        assert!(!HealthStatus::Unhealthy.is_operational());
        
        assert!(!HealthStatus::Healthy.needs_attention());
        assert!(HealthStatus::Degraded.needs_attention());
        assert!(HealthStatus::Unhealthy.needs_attention());
        assert!(HealthStatus::Unknown.needs_attention());
    }

    #[test]
    fn test_deployment_status_checks() {
        assert!(DeploymentStatus::Deployed.is_active());
        assert!(!DeploymentStatus::Planning.is_active());
        
        assert!(DeploymentStatus::Destroyed.is_terminal());
        assert!(DeploymentStatus::Failed.is_terminal());
        assert!(!DeploymentStatus::Deployed.is_terminal());
        
        assert!(DeploymentStatus::Deploying.is_transitional());
        assert!(DeploymentStatus::Updating.is_transitional());
        assert!(!DeploymentStatus::Deployed.is_transitional());
    }

    #[test]
    fn test_component_configuration() {
        let config = ComponentConfiguration::new("2.0".to_string())
            .with_parameter("cpu".to_string(), "2".to_string())
            .with_parameter("memory".to_string(), "4GB".to_string())
            .with_checksum("abc123".to_string());

        assert_eq!(config.schema_version, "2.0");
        assert_eq!(config.get_parameter("cpu"), Some(&"2".to_string()));
        assert_eq!(config.get_parameter("memory"), Some(&"4GB".to_string()));
        assert_eq!(config.checksum, Some("abc123".to_string()));

        assert!(!config.has_changed(Some("abc123")));
        assert!(config.has_changed(Some("def456")));
    }

    #[test]
    fn test_component_resources() {
        let resources = ComponentResources::new()
            .with_cpu("2 cores".to_string())
            .with_memory("4GB".to_string())
            .with_storage("100GB".to_string())
            .with_custom_resource("gpu".to_string(), "1".to_string());

        assert_eq!(resources.cpu, Some("2 cores".to_string()));
        assert_eq!(resources.memory, Some("4GB".to_string()));
        assert_eq!(resources.storage, Some("100GB".to_string()));
        assert_eq!(resources.custom.get("gpu"), Some(&"1".to_string()));
    }

    #[test]
    fn test_infrastructure_component_state() {
        let mut component = InfrastructureComponentState::new(
            "web-server-1".to_string(),
            "Web Server 1".to_string(),
            ComponentType::VirtualMachine,
        )
        .with_version("2.1.0".to_string())
        .with_region("us-west-2".to_string())
        .with_tag("environment".to_string(), "production".to_string());

        assert_eq!(component.component_id, "web-server-1");
        assert_eq!(component.name, "Web Server 1");
        assert_eq!(component.version, "2.1.0");
        assert_eq!(component.region, Some("us-west-2".to_string()));

        component.add_dependency("database-1".to_string());
        component.add_dependent("load-balancer-1".to_string());

        assert!(component.dependencies.contains("database-1"));
        assert!(component.dependents.contains("load-balancer-1"));

        component.update_deployment_status(DeploymentStatus::Deployed);
        component.update_health_status(HealthStatus::Healthy);

        assert_eq!(component.deployment_status, DeploymentStatus::Deployed);
        assert_eq!(component.health_status, HealthStatus::Healthy);
        assert!(component.last_health_check.is_some());
    }

    #[test]
    fn test_component_lifecycle_checks() {
        let mut component = InfrastructureComponentState::new(
            "test-1".to_string(),
            "Test Component".to_string(),
            ComponentType::Service,
        );

        // Initially should be ready for deployment (no dependencies)
        assert!(component.is_ready_for_deployment());

        // Add dependent - should not be able to destroy
        component.add_dependent("dependent-1".to_string());
        assert!(!component.can_be_destroyed());

        // Remove dependent and deploy - should be able to destroy
        component.remove_dependent(&"dependent-1".to_string());
        component.update_deployment_status(DeploymentStatus::Deployed);
        assert!(component.can_be_destroyed());

        // Test health staleness
        let threshold = chrono::Duration::hours(1);
        assert!(component.is_health_stale(threshold)); // No health check yet

        component.update_health_status(HealthStatus::Healthy);
        assert!(!component.is_health_stale(threshold)); // Just checked
    }

    #[test]
    fn test_infrastructure_topology_basic() {
        let mut topology = InfrastructureTopology::new();

        let component1 = InfrastructureComponentState::new(
            "comp-1".to_string(),
            "Component 1".to_string(),
            ComponentType::VirtualMachine,
        );

        let component2 = InfrastructureComponentState::new(
            "comp-2".to_string(),
            "Component 2".to_string(),
            ComponentType::Database,
        );

        topology.add_component(component1);
        topology.add_component(component2);

        assert_eq!(topology.component_count(), 2);
        assert!(topology.get_component(&"comp-1".to_string()).is_some());
        assert!(topology.get_component(&"comp-2".to_string()).is_some());
    }

    #[test]
    fn test_topology_dependencies() {
        let mut topology = InfrastructureTopology::new();

        // Create components
        let web_server = InfrastructureComponentState::new(
            "web".to_string(),
            "Web Server".to_string(),
            ComponentType::VirtualMachine,
        );

        let database = InfrastructureComponentState::new(
            "db".to_string(),
            "Database".to_string(),
            ComponentType::Database,
        );

        topology.add_component(web_server);
        topology.add_component(database);

        // Add dependency: web depends on database
        topology.add_dependency(&"web".to_string(), &"db".to_string()).unwrap();

        let web_comp = topology.get_component(&"web".to_string()).unwrap();
        assert!(web_comp.dependencies.contains("db"));

        let db_comp = topology.get_component(&"db".to_string()).unwrap();
        assert!(db_comp.dependents.contains("web"));
    }

    #[test]
    fn test_topology_circular_dependency_detection() {
        let mut topology = InfrastructureTopology::new();

        let comp_a = InfrastructureComponentState::new("a".to_string(), "A".to_string(), ComponentType::Service);
        let comp_b = InfrastructureComponentState::new("b".to_string(), "B".to_string(), ComponentType::Service);
        let comp_c = InfrastructureComponentState::new("c".to_string(), "C".to_string(), ComponentType::Service);

        topology.add_component(comp_a);
        topology.add_component(comp_b);
        topology.add_component(comp_c);

        // Create chain: A -> B -> C
        topology.add_dependency(&"a".to_string(), &"b".to_string()).unwrap();
        topology.add_dependency(&"b".to_string(), &"c".to_string()).unwrap();

        // Try to create cycle: C -> A (should fail)
        let result = topology.add_dependency(&"c".to_string(), &"a".to_string());
        assert!(result.is_err());

        // Self-dependency should also fail
        let result = topology.add_dependency(&"a".to_string(), &"a".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_topology_deployment_order() {
        let mut topology = InfrastructureTopology::new();

        // Create dependency chain: app -> web -> db
        let app = InfrastructureComponentState::new("app".to_string(), "App".to_string(), ComponentType::Service);
        let web = InfrastructureComponentState::new("web".to_string(), "Web".to_string(), ComponentType::VirtualMachine);
        let db = InfrastructureComponentState::new("db".to_string(), "Database".to_string(), ComponentType::Database);

        topology.add_component(app);
        topology.add_component(web);
        topology.add_component(db);

        topology.add_dependency(&"app".to_string(), &"web".to_string()).unwrap();
        topology.add_dependency(&"web".to_string(), &"db".to_string()).unwrap();

        let deployment_order = topology.get_deployment_order().unwrap();
        
        // Database should be deployed first, then web, then app
        assert_eq!(deployment_order[0], "db");
        assert_eq!(deployment_order[1], "web");
        assert_eq!(deployment_order[2], "app");
    }

    #[test]
    fn test_topology_queries() {
        let mut topology = InfrastructureTopology::new();

        let mut vm1 = InfrastructureComponentState::new("vm1".to_string(), "VM 1".to_string(), ComponentType::VirtualMachine);
        vm1.update_deployment_status(DeploymentStatus::Deployed);
        vm1.update_health_status(HealthStatus::Healthy);

        let mut vm2 = InfrastructureComponentState::new("vm2".to_string(), "VM 2".to_string(), ComponentType::VirtualMachine);
        vm2.update_deployment_status(DeploymentStatus::Failed);
        vm2.update_health_status(HealthStatus::Unhealthy);

        let mut db = InfrastructureComponentState::new("db1".to_string(), "DB 1".to_string(), ComponentType::Database);
        db.update_deployment_status(DeploymentStatus::Deployed);
        db.update_health_status(HealthStatus::Degraded);

        topology.add_component(vm1);
        topology.add_component(vm2);
        topology.add_component(db);

        // Test queries
        let vms = topology.get_components_by_type(&ComponentType::VirtualMachine);
        assert_eq!(vms.len(), 2);

        let deployed = topology.get_components_by_deployment_status(&DeploymentStatus::Deployed);
        assert_eq!(deployed.len(), 2);

        let healthy = topology.get_components_by_health_status(&HealthStatus::Healthy);
        assert_eq!(healthy.len(), 1);

        assert!(!topology.is_healthy()); // Has unhealthy components
    }

    #[test]
    fn test_infrastructure_statistics() {
        let mut topology = InfrastructureTopology::new();

        let mut comp1 = InfrastructureComponentState::new("1".to_string(), "C1".to_string(), ComponentType::VirtualMachine);
        comp1.update_deployment_status(DeploymentStatus::Deployed);
        comp1.update_health_status(HealthStatus::Healthy);

        let mut comp2 = InfrastructureComponentState::new("2".to_string(), "C2".to_string(), ComponentType::VirtualMachine);
        comp2.update_deployment_status(DeploymentStatus::Failed);
        comp2.update_health_status(HealthStatus::Unhealthy);

        let mut comp3 = InfrastructureComponentState::new("3".to_string(), "C3".to_string(), ComponentType::Database);
        comp3.update_deployment_status(DeploymentStatus::Deployed);
        comp3.update_health_status(HealthStatus::Degraded);

        topology.add_component(comp1);
        topology.add_component(comp2);
        topology.add_component(comp3);

        let stats = topology.get_statistics();
        
        assert_eq!(stats.total_components, 3);
        assert_eq!(stats.deployed_components, 2);
        assert_eq!(stats.failed_components, 1);
        assert_eq!(stats.healthy_components, 1);
        assert_eq!(stats.degraded_components, 1);
        assert_eq!(stats.unhealthy_components, 1);

        assert!((stats.deployment_success_rate() - 66.66666666666667).abs() < 0.01);
        assert!((stats.health_success_rate() - 33.333333333333336).abs() < 0.01);
        
        assert_eq!(stats.operational_components(), 2); // healthy + degraded
        assert_eq!(stats.problematic_components(), 2); // unhealthy + failed
        assert_eq!(stats.active_components(), 2); // deployed
        assert_eq!(stats.transitional_components(), 0); // none transitioning

        assert_eq!(stats.components_by_type.get("virtual-machine"), Some(&2));
        assert_eq!(stats.components_by_type.get("database"), Some(&1));
    }

    #[test]
    fn test_stale_health_detection() {
        let mut topology = InfrastructureTopology::new();

        let mut comp = InfrastructureComponentState::new("comp".to_string(), "Component".to_string(), ComponentType::Service);
        comp.update_health_status(HealthStatus::Healthy);
        
        topology.add_component(comp);

        // Should not be stale immediately after health check
        let threshold = chrono::Duration::hours(1);
        let stale = topology.find_stale_health_components(threshold);
        assert_eq!(stale.len(), 0);

        // Test with very short threshold to simulate staleness
        let short_threshold = chrono::Duration::milliseconds(1);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let stale = topology.find_stale_health_components(short_threshold);
        assert_eq!(stale.len(), 1);
    }
}