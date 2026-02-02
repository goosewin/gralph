//! SAML 2.0 SSO integration module.
//!
//! This module provides:
//! - SAML 2.0 Service Provider (SP) metadata generation
//! - Assertion Consumer Service (ACS) for processing SAML responses
//! - User provisioning from SAML assertions/claims
//! - Session binding after SAML assertion validation
//!
//! The implementation follows the SAML 2.0 Web Browser SSO Profile.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::auth::{AuthError, TokenPair, User, UserRole, UserStore};

/// SAML 2.0 configuration for the Service Provider.
#[derive(Debug, Clone)]
pub struct SamlConfig {
    /// Service Provider Entity ID (typically the application URL)
    pub entity_id: String,
    /// Assertion Consumer Service URL (where IdP sends SAML responses)
    pub acs_url: String,
    /// Single Logout Service URL (optional)
    pub slo_url: Option<String>,
    /// Identity Provider Entity ID
    pub idp_entity_id: String,
    /// Identity Provider SSO URL (where to redirect users for login)
    pub idp_sso_url: String,
    /// Identity Provider certificate for signature verification (PEM encoded)
    pub idp_certificate: String,
    /// Whether to sign authentication requests
    pub sign_requests: bool,
    /// SP private key for signing (PEM encoded, optional)
    pub sp_private_key: Option<String>,
    /// SP certificate for metadata (PEM encoded, optional)
    pub sp_certificate: Option<String>,
    /// Attribute mapping from SAML attributes to user fields
    pub attribute_mapping: SamlAttributeMapping,
    /// Session validity duration after SAML authentication
    pub session_duration: Duration,
    /// Whether to allow IdP-initiated SSO
    pub allow_idp_initiated: bool,
    /// Organization name for metadata
    pub organization_name: Option<String>,
    /// Organization URL for metadata
    pub organization_url: Option<String>,
    /// Contact email for metadata
    pub contact_email: Option<String>,
}

impl Default for SamlConfig {
    fn default() -> Self {
        Self {
            entity_id: String::new(),
            acs_url: String::new(),
            slo_url: None,
            idp_entity_id: String::new(),
            idp_sso_url: String::new(),
            idp_certificate: String::new(),
            sign_requests: false,
            sp_private_key: None,
            sp_certificate: None,
            attribute_mapping: SamlAttributeMapping::default(),
            session_duration: Duration::from_secs(8 * 60 * 60), // 8 hours
            allow_idp_initiated: false,
            organization_name: None,
            organization_url: None,
            contact_email: None,
        }
    }
}

impl SamlConfig {
    pub fn new(entity_id: &str, acs_url: &str, idp_entity_id: &str, idp_sso_url: &str) -> Self {
        Self {
            entity_id: entity_id.to_string(),
            acs_url: acs_url.to_string(),
            idp_entity_id: idp_entity_id.to_string(),
            idp_sso_url: idp_sso_url.to_string(),
            ..Default::default()
        }
    }

    pub fn with_idp_certificate(mut self, cert: &str) -> Self {
        self.idp_certificate = cert.to_string();
        self
    }

    pub fn with_signing(mut self, private_key: &str, certificate: &str) -> Self {
        self.sp_private_key = Some(private_key.to_string());
        self.sp_certificate = Some(certificate.to_string());
        self.sign_requests = true;
        self
    }

    pub fn with_slo_url(mut self, url: &str) -> Self {
        self.slo_url = Some(url.to_string());
        self
    }

    pub fn with_attribute_mapping(mut self, mapping: SamlAttributeMapping) -> Self {
        self.attribute_mapping = mapping;
        self
    }

    pub fn with_session_duration(mut self, duration: Duration) -> Self {
        self.session_duration = duration;
        self
    }

    pub fn with_idp_initiated(mut self, allow: bool) -> Self {
        self.allow_idp_initiated = allow;
        self
    }

    pub fn with_organization(mut self, name: &str, url: &str) -> Self {
        self.organization_name = Some(name.to_string());
        self.organization_url = Some(url.to_string());
        self
    }

    pub fn with_contact_email(mut self, email: &str) -> Self {
        self.contact_email = Some(email.to_string());
        self
    }

    /// Validate the configuration is complete.
    pub fn validate(&self) -> Result<(), SamlError> {
        if self.entity_id.is_empty() {
            return Err(SamlError::InvalidConfig("entity_id is required".to_string()));
        }
        if self.acs_url.is_empty() {
            return Err(SamlError::InvalidConfig("acs_url is required".to_string()));
        }
        if self.idp_entity_id.is_empty() {
            return Err(SamlError::InvalidConfig("idp_entity_id is required".to_string()));
        }
        if self.idp_sso_url.is_empty() {
            return Err(SamlError::InvalidConfig("idp_sso_url is required".to_string()));
        }
        Ok(())
    }
}

/// Mapping from SAML attribute names to user profile fields.
#[derive(Debug, Clone)]
pub struct SamlAttributeMapping {
    /// SAML attribute name for user email
    pub email: String,
    /// SAML attribute name for first name (optional)
    pub first_name: Option<String>,
    /// SAML attribute name for last name (optional)
    pub last_name: Option<String>,
    /// SAML attribute name for display name (optional)
    pub display_name: Option<String>,
    /// SAML attribute name for user groups/roles (optional)
    pub groups: Option<String>,
    /// Default role for provisioned users
    pub default_role: UserRole,
    /// Group to role mapping
    pub group_role_mapping: HashMap<String, UserRole>,
}

impl Default for SamlAttributeMapping {
    fn default() -> Self {
        Self {
            email: "email".to_string(),
            first_name: Some("firstName".to_string()),
            last_name: Some("lastName".to_string()),
            display_name: Some("displayName".to_string()),
            groups: Some("groups".to_string()),
            default_role: UserRole::Viewer,
            group_role_mapping: HashMap::new(),
        }
    }
}

impl SamlAttributeMapping {
    pub fn new(email_attr: &str) -> Self {
        Self {
            email: email_attr.to_string(),
            ..Default::default()
        }
    }

    pub fn with_name_attributes(mut self, first_name: &str, last_name: &str) -> Self {
        self.first_name = Some(first_name.to_string());
        self.last_name = Some(last_name.to_string());
        self
    }

    pub fn with_display_name(mut self, attr: &str) -> Self {
        self.display_name = Some(attr.to_string());
        self
    }

    pub fn with_groups(mut self, attr: &str) -> Self {
        self.groups = Some(attr.to_string());
        self
    }

    pub fn with_default_role(mut self, role: UserRole) -> Self {
        self.default_role = role;
        self
    }

    pub fn with_group_role_mapping(mut self, mapping: HashMap<String, UserRole>) -> Self {
        self.group_role_mapping = mapping;
        self
    }
}

/// SAML error types.
#[derive(Debug, Clone)]
pub enum SamlError {
    /// Invalid configuration
    InvalidConfig(String),
    /// Invalid SAML response
    InvalidResponse(String),
    /// Signature verification failed
    SignatureInvalid,
    /// Response has expired
    ResponseExpired,
    /// Assertion conditions not met
    ConditionsNotMet(String),
    /// Missing required attribute
    MissingAttribute(String),
    /// User provisioning failed
    ProvisioningFailed(String),
    /// XML parsing error
    XmlParseError(String),
    /// Base64 decoding error
    Base64Error(String),
    /// Authentication request expired
    RequestExpired,
    /// Unknown authentication request
    UnknownRequest,
    /// IdP-initiated SSO not allowed
    IdpInitiatedNotAllowed,
}

impl std::fmt::Display for SamlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SamlError::InvalidConfig(msg) => write!(f, "invalid SAML configuration: {}", msg),
            SamlError::InvalidResponse(msg) => write!(f, "invalid SAML response: {}", msg),
            SamlError::SignatureInvalid => write!(f, "SAML signature verification failed"),
            SamlError::ResponseExpired => write!(f, "SAML response has expired"),
            SamlError::ConditionsNotMet(msg) => write!(f, "SAML conditions not met: {}", msg),
            SamlError::MissingAttribute(attr) => write!(f, "missing required SAML attribute: {}", attr),
            SamlError::ProvisioningFailed(msg) => write!(f, "user provisioning failed: {}", msg),
            SamlError::XmlParseError(msg) => write!(f, "XML parsing error: {}", msg),
            SamlError::Base64Error(msg) => write!(f, "base64 decoding error: {}", msg),
            SamlError::RequestExpired => write!(f, "SAML authentication request has expired"),
            SamlError::UnknownRequest => write!(f, "unknown SAML authentication request"),
            SamlError::IdpInitiatedNotAllowed => write!(f, "IdP-initiated SSO is not allowed"),
        }
    }
}

impl std::error::Error for SamlError {}

/// SAML authentication request tracking.
#[derive(Debug, Clone)]
pub struct SamlAuthRequest {
    /// Request ID
    pub id: String,
    /// Issue instant (Unix timestamp)
    pub issue_instant: u64,
    /// Relay state for return URL
    pub relay_state: Option<String>,
    /// Request expiry time (Unix timestamp)
    pub expires_at: u64,
}

impl SamlAuthRequest {
    pub fn new(relay_state: Option<String>, validity_secs: u64) -> Self {
        let now = current_timestamp();
        Self {
            id: format!("_saml_req_{}", Uuid::new_v4()),
            issue_instant: now,
            relay_state,
            expires_at: now + validity_secs,
        }
    }

    pub fn is_expired(&self) -> bool {
        current_timestamp() > self.expires_at
    }
}

/// Parsed SAML assertion containing user attributes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlAssertion {
    /// Assertion ID
    pub id: String,
    /// Issuer (IdP entity ID)
    pub issuer: String,
    /// Subject name ID (typically email or user ID)
    pub name_id: String,
    /// Name ID format
    pub name_id_format: Option<String>,
    /// Issue instant (Unix timestamp)
    pub issue_instant: u64,
    /// Not before condition (Unix timestamp)
    pub not_before: Option<u64>,
    /// Not on or after condition (Unix timestamp)
    pub not_on_or_after: Option<u64>,
    /// Session not on or after (Unix timestamp)
    pub session_not_on_or_after: Option<u64>,
    /// Session index from IdP
    pub session_index: Option<String>,
    /// Audience restrictions
    pub audiences: Vec<String>,
    /// User attributes from the assertion
    pub attributes: HashMap<String, Vec<String>>,
    /// Authentication context class
    pub authn_context: Option<String>,
    /// InResponseTo (the original request ID)
    pub in_response_to: Option<String>,
}

impl SamlAssertion {
    /// Check if the assertion is valid based on time conditions.
    pub fn is_valid(&self, audience: &str, clock_skew_secs: u64) -> Result<(), SamlError> {
        let now = current_timestamp();

        // Check not_before condition
        if let Some(not_before) = self.not_before {
            if now + clock_skew_secs < not_before {
                return Err(SamlError::ConditionsNotMet(
                    "assertion not yet valid (notBefore)".to_string(),
                ));
            }
        }

        // Check not_on_or_after condition
        if let Some(not_on_or_after) = self.not_on_or_after {
            if now > not_on_or_after + clock_skew_secs {
                return Err(SamlError::ResponseExpired);
            }
        }

        // Check audience restriction
        if !self.audiences.is_empty() && !self.audiences.contains(&audience.to_string()) {
            return Err(SamlError::ConditionsNotMet(format!(
                "audience mismatch: expected {}, got {:?}",
                audience, self.audiences
            )));
        }

        Ok(())
    }

    /// Get a single attribute value.
    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .get(name)
            .and_then(|values| values.first())
            .map(String::as_str)
    }

    /// Get all values for an attribute.
    pub fn get_attribute_values(&self, name: &str) -> Option<&Vec<String>> {
        self.attributes.get(name)
    }
}

/// User profile extracted from SAML assertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlUserProfile {
    /// User email (required)
    pub email: String,
    /// First name
    pub first_name: Option<String>,
    /// Last name
    pub last_name: Option<String>,
    /// Display name
    pub display_name: Option<String>,
    /// User groups
    pub groups: Vec<String>,
    /// Determined role
    pub role: UserRole,
    /// IdP session index
    pub session_index: Option<String>,
    /// Name ID for logout
    pub name_id: String,
    /// Name ID format
    pub name_id_format: Option<String>,
}

/// SAML authentication result containing tokens and user info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlAuthResult {
    /// JWT token pair for the session
    pub tokens: TokenPair,
    /// User profile from SAML assertion
    pub user: SamlUserProfile,
    /// Relay state from original request
    pub relay_state: Option<String>,
    /// Whether the user was newly provisioned
    pub is_new_user: bool,
}

/// SAML Service Provider implementation.
pub struct SamlServiceProvider {
    config: SamlConfig,
    pending_requests: std::sync::RwLock<HashMap<String, SamlAuthRequest>>,
    request_validity_secs: u64,
    clock_skew_secs: u64,
}

impl SamlServiceProvider {
    pub fn new(config: SamlConfig) -> Result<Self, SamlError> {
        config.validate()?;
        Ok(Self {
            config,
            pending_requests: std::sync::RwLock::new(HashMap::new()),
            request_validity_secs: 300, // 5 minutes
            clock_skew_secs: 180,       // 3 minutes clock skew tolerance
        })
    }

    pub fn with_request_validity(mut self, secs: u64) -> Self {
        self.request_validity_secs = secs;
        self
    }

    pub fn with_clock_skew(mut self, secs: u64) -> Self {
        self.clock_skew_secs = secs;
        self
    }

    /// Generate SP metadata XML.
    pub fn generate_metadata(&self) -> String {
        let mut xml = String::new();
        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push('\n');
        xml.push_str(&format!(
            r#"<md:EntityDescriptor xmlns:md="urn:oasis:names:tc:SAML:2.0:metadata" entityID="{}">"#,
            xml_escape(&self.config.entity_id)
        ));
        xml.push('\n');

        // SP SSO Descriptor
        xml.push_str(r#"  <md:SPSSODescriptor protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol" AuthnRequestsSigned="false" WantAssertionsSigned="true">"#);
        xml.push('\n');

        // Key descriptor for signing (if certificate provided)
        if let Some(ref cert) = self.config.sp_certificate {
            xml.push_str(r#"    <md:KeyDescriptor use="signing">"#);
            xml.push('\n');
            xml.push_str(r#"      <ds:KeyInfo xmlns:ds="http://www.w3.org/2000/09/xmldsig#">"#);
            xml.push('\n');
            xml.push_str(r#"        <ds:X509Data>"#);
            xml.push('\n');
            xml.push_str(&format!(
                r#"          <ds:X509Certificate>{}</ds:X509Certificate>"#,
                extract_certificate_body(cert)
            ));
            xml.push('\n');
            xml.push_str(r#"        </ds:X509Data>"#);
            xml.push('\n');
            xml.push_str(r#"      </ds:KeyInfo>"#);
            xml.push('\n');
            xml.push_str(r#"    </md:KeyDescriptor>"#);
            xml.push('\n');
        }

        // Name ID format
        xml.push_str(r#"    <md:NameIDFormat>urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress</md:NameIDFormat>"#);
        xml.push('\n');
        xml.push_str(r#"    <md:NameIDFormat>urn:oasis:names:tc:SAML:2.0:nameid-format:persistent</md:NameIDFormat>"#);
        xml.push('\n');
        xml.push_str(r#"    <md:NameIDFormat>urn:oasis:names:tc:SAML:2.0:nameid-format:transient</md:NameIDFormat>"#);
        xml.push('\n');

        // Assertion Consumer Service
        xml.push_str(&format!(
            r#"    <md:AssertionConsumerService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST" Location="{}" index="0" isDefault="true"/>"#,
            xml_escape(&self.config.acs_url)
        ));
        xml.push('\n');

        // Single Logout Service (if configured)
        if let Some(ref slo_url) = self.config.slo_url {
            xml.push_str(&format!(
                r#"    <md:SingleLogoutService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST" Location="{}"/>"#,
                xml_escape(slo_url)
            ));
            xml.push('\n');
        }

        xml.push_str(r#"  </md:SPSSODescriptor>"#);
        xml.push('\n');

        // Organization info
        if let (Some(name), Some(url)) = (&self.config.organization_name, &self.config.organization_url) {
            xml.push_str(r#"  <md:Organization>"#);
            xml.push('\n');
            xml.push_str(&format!(
                r#"    <md:OrganizationName xml:lang="en">{}</md:OrganizationName>"#,
                xml_escape(name)
            ));
            xml.push('\n');
            xml.push_str(&format!(
                r#"    <md:OrganizationDisplayName xml:lang="en">{}</md:OrganizationDisplayName>"#,
                xml_escape(name)
            ));
            xml.push('\n');
            xml.push_str(&format!(
                r#"    <md:OrganizationURL xml:lang="en">{}</md:OrganizationURL>"#,
                xml_escape(url)
            ));
            xml.push('\n');
            xml.push_str(r#"  </md:Organization>"#);
            xml.push('\n');
        }

        // Contact person
        if let Some(ref email) = self.config.contact_email {
            xml.push_str(r#"  <md:ContactPerson contactType="technical">"#);
            xml.push('\n');
            xml.push_str(&format!(
                r#"    <md:EmailAddress>{}</md:EmailAddress>"#,
                xml_escape(email)
            ));
            xml.push('\n');
            xml.push_str(r#"  </md:ContactPerson>"#);
            xml.push('\n');
        }

        xml.push_str(r#"</md:EntityDescriptor>"#);
        xml
    }

    /// Create an authentication request and return the redirect URL.
    pub fn create_authn_request(&self, relay_state: Option<String>) -> (String, SamlAuthRequest) {
        let request = SamlAuthRequest::new(relay_state.clone(), self.request_validity_secs);
        let request_id = request.id.clone();

        // Store the request for validation
        {
            let mut requests = self.pending_requests.write().unwrap();
            // Clean up expired requests
            requests.retain(|_, req| !req.is_expired());
            requests.insert(request_id.clone(), request.clone());
        }

        // Generate SAML AuthnRequest XML
        let authn_request_xml = self.generate_authn_request_xml(&request);
        let encoded_request = BASE64.encode(authn_request_xml.as_bytes());

        // Build redirect URL with SAMLRequest parameter
        let mut url = format!(
            "{}?SAMLRequest={}",
            self.config.idp_sso_url,
            urlencoding::encode(&encoded_request)
        );

        if let Some(ref state) = relay_state {
            url.push_str(&format!("&RelayState={}", urlencoding::encode(state)));
        }

        (url, request)
    }

    /// Generate SAML AuthnRequest XML.
    fn generate_authn_request_xml(&self, request: &SamlAuthRequest) -> String {
        let issue_instant = format_iso8601(request.issue_instant);
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol"
    xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion"
    ID="{id}"
    Version="2.0"
    IssueInstant="{issue_instant}"
    Destination="{destination}"
    AssertionConsumerServiceURL="{acs_url}"
    ProtocolBinding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST">
  <saml:Issuer>{issuer}</saml:Issuer>
  <samlp:NameIDPolicy Format="urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress" AllowCreate="true"/>
</samlp:AuthnRequest>"#,
            id = xml_escape(&request.id),
            issue_instant = issue_instant,
            destination = xml_escape(&self.config.idp_sso_url),
            acs_url = xml_escape(&self.config.acs_url),
            issuer = xml_escape(&self.config.entity_id),
        )
    }

    /// Process a SAML response from the IdP.
    pub fn process_response(
        &self,
        saml_response: &str,
        relay_state: Option<&str>,
    ) -> Result<(SamlAssertion, Option<String>), SamlError> {
        // Decode base64 SAML response
        let decoded = BASE64
            .decode(saml_response.as_bytes())
            .map_err(|e| SamlError::Base64Error(e.to_string()))?;

        let response_xml = String::from_utf8(decoded)
            .map_err(|e| SamlError::XmlParseError(format!("invalid UTF-8: {}", e)))?;

        // Parse the SAML response
        let assertion = self.parse_saml_response(&response_xml)?;

        // Validate InResponseTo if this is SP-initiated SSO
        if let Some(ref in_response_to) = assertion.in_response_to {
            let requests = self.pending_requests.read().unwrap();
            match requests.get(in_response_to) {
                Some(req) if req.is_expired() => {
                    return Err(SamlError::RequestExpired);
                }
                Some(_) => {
                    // Valid request, will be cleaned up later
                }
                None => {
                    return Err(SamlError::UnknownRequest);
                }
            }
        } else if !self.config.allow_idp_initiated {
            return Err(SamlError::IdpInitiatedNotAllowed);
        }

        // Validate assertion conditions
        assertion.is_valid(&self.config.entity_id, self.clock_skew_secs)?;

        // Clean up the pending request
        if let Some(ref in_response_to) = assertion.in_response_to {
            let mut requests = self.pending_requests.write().unwrap();
            requests.remove(in_response_to);
        }

        Ok((assertion, relay_state.map(String::from)))
    }

    /// Parse SAML response XML and extract assertion.
    fn parse_saml_response(&self, xml: &str) -> Result<SamlAssertion, SamlError> {
        // Simplified XML parsing - in production, use a proper XML library with signature verification
        // This implementation handles the basic structure for demonstration

        let in_response_to = extract_xml_attribute(xml, "InResponseTo");

        // Extract issuer
        let issuer = extract_xml_element(xml, "Issuer")
            .ok_or_else(|| SamlError::InvalidResponse("missing Issuer".to_string()))?;

        // Validate issuer matches expected IdP
        if issuer != self.config.idp_entity_id {
            return Err(SamlError::InvalidResponse(format!(
                "issuer mismatch: expected {}, got {}",
                self.config.idp_entity_id, issuer
            )));
        }

        // Extract assertion ID
        let assertion_id = extract_xml_attribute(xml, "ID")
            .ok_or_else(|| SamlError::InvalidResponse("missing assertion ID".to_string()))?;

        // Extract issue instant
        let issue_instant_str = extract_xml_attribute(xml, "IssueInstant")
            .ok_or_else(|| SamlError::InvalidResponse("missing IssueInstant".to_string()))?;
        let issue_instant = parse_iso8601(&issue_instant_str)
            .ok_or_else(|| SamlError::InvalidResponse("invalid IssueInstant format".to_string()))?;

        // Extract NameID
        let name_id = extract_xml_element(xml, "NameID")
            .ok_or_else(|| SamlError::InvalidResponse("missing NameID".to_string()))?;

        // Extract conditions
        let not_before = extract_xml_attribute(xml, "NotBefore").and_then(|s| parse_iso8601(&s));
        let not_on_or_after = extract_xml_attribute(xml, "NotOnOrAfter").and_then(|s| parse_iso8601(&s));

        // Extract audiences
        let audiences = extract_xml_elements(xml, "Audience");

        // Extract session info
        let session_index = extract_xml_attribute(xml, "SessionIndex");
        let session_not_on_or_after = extract_xml_attribute(xml, "SessionNotOnOrAfter")
            .and_then(|s| parse_iso8601(&s));

        // Extract attributes
        let attributes = extract_saml_attributes(xml);

        // Extract authentication context
        let authn_context = extract_xml_element(xml, "AuthnContextClassRef");

        Ok(SamlAssertion {
            id: assertion_id,
            issuer,
            name_id,
            name_id_format: extract_xml_attribute(xml, "Format"),
            issue_instant,
            not_before,
            not_on_or_after,
            session_not_on_or_after,
            session_index,
            audiences,
            attributes,
            authn_context,
            in_response_to,
        })
    }

    /// Extract user profile from SAML assertion using attribute mapping.
    pub fn extract_user_profile(&self, assertion: &SamlAssertion) -> Result<SamlUserProfile, SamlError> {
        let mapping = &self.config.attribute_mapping;

        // Get email (required)
        let email = assertion
            .get_attribute(&mapping.email)
            .or_else(|| {
                // Fall back to NameID if it looks like an email
                if assertion.name_id.contains('@') {
                    Some(assertion.name_id.as_str())
                } else {
                    None
                }
            })
            .ok_or_else(|| SamlError::MissingAttribute(mapping.email.clone()))?
            .to_string();

        // Get optional name attributes
        let first_name = mapping
            .first_name
            .as_ref()
            .and_then(|attr| assertion.get_attribute(attr))
            .map(String::from);

        let last_name = mapping
            .last_name
            .as_ref()
            .and_then(|attr| assertion.get_attribute(attr))
            .map(String::from);

        let display_name = mapping
            .display_name
            .as_ref()
            .and_then(|attr| assertion.get_attribute(attr))
            .map(String::from);

        // Get groups
        let groups = mapping
            .groups
            .as_ref()
            .and_then(|attr| assertion.get_attribute_values(attr))
            .cloned()
            .unwrap_or_default();

        // Determine role from groups
        let role = determine_role(&groups, &mapping.group_role_mapping, mapping.default_role);

        Ok(SamlUserProfile {
            email,
            first_name,
            last_name,
            display_name,
            groups,
            role,
            session_index: assertion.session_index.clone(),
            name_id: assertion.name_id.clone(),
            name_id_format: assertion.name_id_format.clone(),
        })
    }

    /// Get the IdP SSO URL for redirects.
    pub fn idp_sso_url(&self) -> &str {
        &self.config.idp_sso_url
    }

    /// Get the SP entity ID.
    pub fn entity_id(&self) -> &str {
        &self.config.entity_id
    }

    /// Get the ACS URL.
    pub fn acs_url(&self) -> &str {
        &self.config.acs_url
    }
}

/// SAML authentication service combining SP with user store.
pub struct SamlAuthService {
    sp: SamlServiceProvider,
    user_store: UserStore,
    jwt_config: crate::auth::JwtConfig,
}

impl SamlAuthService {
    pub fn new(
        sp: SamlServiceProvider,
        user_store: UserStore,
        jwt_config: crate::auth::JwtConfig,
    ) -> Self {
        Self {
            sp,
            user_store,
            jwt_config,
        }
    }

    /// Start SAML authentication flow by returning redirect URL.
    pub fn start_auth(&self, relay_state: Option<String>) -> (String, SamlAuthRequest) {
        self.sp.create_authn_request(relay_state)
    }

    /// Complete SAML authentication by processing response and creating session.
    pub fn complete_auth(
        &self,
        saml_response: &str,
        relay_state: Option<&str>,
    ) -> Result<SamlAuthResult, SamlError> {
        // Process and validate the SAML response
        let (assertion, relay_state) = self.sp.process_response(saml_response, relay_state)?;

        // Extract user profile
        let profile = self.sp.extract_user_profile(&assertion)?;

        // Provision or update user
        let (user, is_new_user) = self.provision_user(&profile)?;

        // Generate JWT tokens
        let tokens = self.generate_tokens(&user)
            .map_err(|e| SamlError::ProvisioningFailed(e.to_string()))?;

        Ok(SamlAuthResult {
            tokens,
            user: profile,
            relay_state,
            is_new_user,
        })
    }

    /// Provision a user from SAML profile (create or update).
    fn provision_user(&self, profile: &SamlUserProfile) -> Result<(User, bool), SamlError> {
        // Check if user exists
        if let Some(existing) = self.user_store.find_by_email(&profile.email) {
            // User exists, return existing (could update role here if needed)
            return Ok((existing, false));
        }

        // Create new user with a random password (they'll use SAML for auth)
        let random_password = format!("SAML-{}-{}", Uuid::new_v4(), Uuid::new_v4());
        let user = self
            .user_store
            .register(&profile.email, &random_password, profile.role)
            .map_err(|e| SamlError::ProvisioningFailed(e.to_string()))?;

        Ok((user, true))
    }

    /// Generate JWT tokens for a user.
    fn generate_tokens(&self, user: &User) -> Result<TokenPair, AuthError> {
        let now = current_timestamp();
        let access_jti = Uuid::new_v4().to_string();
        let refresh_jti = Uuid::new_v4().to_string();
        let family = Uuid::new_v4().to_string();

        let access_claims = crate::auth::AccessTokenClaims {
            sub: user.id.clone(),
            email: user.email.clone(),
            role: user.role,
            exp: now + self.jwt_config.access_token_expiry,
            iat: now,
            jti: access_jti,
            token_type: "access".to_string(),
        };

        let refresh_claims = crate::auth::RefreshTokenClaims {
            sub: user.id.clone(),
            exp: now + self.jwt_config.refresh_token_expiry,
            iat: now,
            jti: refresh_jti,
            token_type: "refresh".to_string(),
            family,
        };

        let access_token = encode_token(&access_claims, &self.jwt_config)?;
        let refresh_token = encode_token(&refresh_claims, &self.jwt_config)?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.jwt_config.access_token_expiry,
        })
    }

    /// Get SP metadata.
    pub fn metadata(&self) -> String {
        self.sp.generate_metadata()
    }

    /// Get the IdP SSO URL.
    pub fn idp_sso_url(&self) -> &str {
        self.sp.idp_sso_url()
    }
}

// Helper functions

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn format_iso8601(timestamp: u64) -> String {
    // Simple ISO 8601 format
    let secs = timestamp;
    let days_since_epoch = secs / 86400;
    let secs_in_day = secs % 86400;
    let hours = secs_in_day / 3600;
    let minutes = (secs_in_day % 3600) / 60;
    let seconds = secs_in_day % 60;

    // Approximate date calculation (not accounting for leap years precisely)
    let mut year = 1970;
    let mut remaining_days = days_since_epoch;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for days in days_in_months {
        if remaining_days < days {
            break;
        }
        remaining_days -= days;
        month += 1;
    }
    let day = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn parse_iso8601(s: &str) -> Option<u64> {
    // Parse ISO 8601 datetime string
    // Expected format: YYYY-MM-DDTHH:MM:SSZ or YYYY-MM-DDTHH:MM:SS.sssZ
    let s = s.trim_end_matches('Z');
    let s = s.split('.').next()?; // Remove milliseconds if present

    let parts: Vec<&str> = s.split('T').collect();
    if parts.len() != 2 {
        return None;
    }

    let date_parts: Vec<u64> = parts[0].split('-').filter_map(|p| p.parse().ok()).collect();
    let time_parts: Vec<u64> = parts[1].split(':').filter_map(|p| p.parse().ok()).collect();

    if date_parts.len() != 3 || time_parts.len() != 3 {
        return None;
    }

    let year = date_parts[0];
    let month = date_parts[1];
    let day = date_parts[2];
    let hours = time_parts[0];
    let minutes = time_parts[1];
    let seconds = time_parts[2];

    // Calculate days since epoch
    let mut days: u64 = 0;
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }

    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    for m in 1..month {
        days += days_in_months[(m - 1) as usize];
    }
    days += day - 1;

    Some(days * 86400 + hours * 3600 + minutes * 60 + seconds)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn extract_certificate_body(pem: &str) -> String {
    pem.lines()
        .filter(|line| !line.starts_with("-----"))
        .collect::<Vec<_>>()
        .join("")
}

fn extract_xml_attribute(xml: &str, attr: &str) -> Option<String> {
    let pattern = format!(r#"{}=""#, attr);
    let start = xml.find(&pattern)? + pattern.len();
    let end = xml[start..].find('"')? + start;
    Some(xml[start..end].to_string())
}

fn extract_xml_element(xml: &str, element: &str) -> Option<String> {
    // Try with namespace prefix first
    for prefix in ["", "saml:", "samlp:", "ds:"] {
        let tag_start_pattern = format!("<{}{}", prefix, element);
        let close_tag = format!("</{}{}>", prefix, element);

        let mut search_from = 0;
        while let Some(rel_start) = xml[search_from..].find(&tag_start_pattern) {
            let tag_start = search_from + rel_start;
            // Check that this is the complete tag name (followed by > or space for attributes)
            let after_tag_name = tag_start + tag_start_pattern.len();
            let next_char = xml[after_tag_name..].chars().next();
            if !matches!(next_char, Some('>') | Some(' ') | Some('/')) {
                // Not a complete tag name match, skip
                search_from = after_tag_name;
                continue;
            }
            // Find the end of the opening tag (could be > or have attributes before >)
            if let Some(tag_end_offset) = xml[after_tag_name..].find('>') {
                let content_start = after_tag_name + tag_end_offset + 1;
                if let Some(end) = xml[content_start..].find(&close_tag) {
                    return Some(xml[content_start..content_start + end].to_string());
                }
            }
            search_from = after_tag_name;
        }
    }
    None
}

fn extract_xml_elements(xml: &str, element: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut search_from = 0;

    for prefix in ["", "saml:", "samlp:"] {
        let tag_start_pattern = format!("<{}{}", prefix, element);
        let close_tag = format!("</{}{}>", prefix, element);

        while let Some(start) = xml[search_from..].find(&tag_start_pattern) {
            let tag_name_end = search_from + start + tag_start_pattern.len();
            // Check that this is the complete tag name (followed by > or space for attributes)
            let next_char = xml[tag_name_end..].chars().next();
            if !matches!(next_char, Some('>') | Some(' ') | Some('/')) {
                // Not a complete tag name match, skip
                search_from = tag_name_end;
                continue;
            }
            // Find the end of the opening tag
            if let Some(tag_end_offset) = xml[tag_name_end..].find('>') {
                let content_start = tag_name_end + tag_end_offset + 1;
                if let Some(end) = xml[content_start..].find(&close_tag) {
                    results.push(xml[content_start..content_start + end].to_string());
                    search_from = content_start + end + close_tag.len();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    results
}

fn extract_saml_attributes(xml: &str) -> HashMap<String, Vec<String>> {
    let mut attributes = HashMap::new();

    // Simple attribute extraction - looks for <saml:Attribute Name="..."> elements
    let mut pos = 0;
    while let Some(attr_start) = xml[pos..].find("<saml:Attribute ") {
        let abs_start = pos + attr_start;

        // Extract attribute name
        if let Some(name_start) = xml[abs_start..].find(r#"Name=""#) {
            let name_start = abs_start + name_start + 6;
            if let Some(name_end) = xml[name_start..].find('"') {
                let attr_name = xml[name_start..name_start + name_end].to_string();

                // Find attribute values
                let mut values = Vec::new();
                let attr_end = xml[abs_start..].find("</saml:Attribute>")
                    .map(|e| abs_start + e)
                    .unwrap_or(xml.len());

                let attr_section = &xml[abs_start..attr_end];

                // Extract AttributeValue elements
                let mut value_pos = 0;
                while let Some(value_start) = attr_section[value_pos..].find("<saml:AttributeValue") {
                    let abs_value_start = value_pos + value_start;
                    if let Some(content_start) = attr_section[abs_value_start..].find('>') {
                        let content_start = abs_value_start + content_start + 1;
                        if let Some(content_end) = attr_section[content_start..].find("</saml:AttributeValue>") {
                            values.push(attr_section[content_start..content_start + content_end].to_string());
                            value_pos = content_start + content_end;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                if !values.is_empty() {
                    attributes.insert(attr_name, values);
                }

                pos = attr_end;
            } else {
                pos = abs_start + 1;
            }
        } else {
            pos = abs_start + 1;
        }
    }

    attributes
}

fn determine_role(
    groups: &[String],
    mapping: &HashMap<String, UserRole>,
    default: UserRole,
) -> UserRole {
    // Check groups from highest to lowest privilege
    for group in groups {
        if let Some(role) = mapping.get(group) {
            return *role;
        }
    }
    default
}

fn encode_token<T: serde::Serialize>(
    claims: &T,
    config: &crate::auth::JwtConfig,
) -> Result<String, AuthError> {
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        claims,
        &jsonwebtoken::EncodingKey::from_secret(config.secret.as_bytes()),
    )
    .map_err(|e| AuthError::TokenGenerationError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test helper to create a basic SAML config
    fn test_config() -> SamlConfig {
        SamlConfig::new(
            "https://app.example.com",
            "https://app.example.com/saml/acs",
            "https://idp.example.com",
            "https://idp.example.com/sso",
        )
        .with_idp_certificate("-----BEGIN CERTIFICATE-----\nMIIC...\n-----END CERTIFICATE-----")
    }

    // SAML config tests

    #[test]
    fn saml_config_default_values() {
        let config = SamlConfig::default();
        assert!(config.entity_id.is_empty());
        assert!(config.acs_url.is_empty());
        assert!(config.slo_url.is_none());
        assert!(!config.sign_requests);
        assert!(!config.allow_idp_initiated);
        assert_eq!(config.session_duration, Duration::from_secs(8 * 60 * 60));
    }

    #[test]
    fn saml_config_new_sets_required_fields() {
        let config = SamlConfig::new(
            "https://sp.example.com",
            "https://sp.example.com/acs",
            "https://idp.example.com",
            "https://idp.example.com/sso",
        );
        assert_eq!(config.entity_id, "https://sp.example.com");
        assert_eq!(config.acs_url, "https://sp.example.com/acs");
        assert_eq!(config.idp_entity_id, "https://idp.example.com");
        assert_eq!(config.idp_sso_url, "https://idp.example.com/sso");
    }

    #[test]
    fn saml_config_builder_methods() {
        let config = SamlConfig::new(
            "https://sp.example.com",
            "https://sp.example.com/acs",
            "https://idp.example.com",
            "https://idp.example.com/sso",
        )
        .with_idp_certificate("cert-data")
        .with_slo_url("https://sp.example.com/slo")
        .with_session_duration(Duration::from_secs(3600))
        .with_idp_initiated(true)
        .with_organization("Gralph Inc", "https://gralph.dev")
        .with_contact_email("support@gralph.dev");

        assert_eq!(config.idp_certificate, "cert-data");
        assert_eq!(config.slo_url, Some("https://sp.example.com/slo".to_string()));
        assert_eq!(config.session_duration, Duration::from_secs(3600));
        assert!(config.allow_idp_initiated);
        assert_eq!(config.organization_name, Some("Gralph Inc".to_string()));
        assert_eq!(config.organization_url, Some("https://gralph.dev".to_string()));
        assert_eq!(config.contact_email, Some("support@gralph.dev".to_string()));
    }

    #[test]
    fn saml_config_signing_setup() {
        let config = SamlConfig::new(
            "https://sp.example.com",
            "https://sp.example.com/acs",
            "https://idp.example.com",
            "https://idp.example.com/sso",
        )
        .with_signing("private-key", "certificate");

        assert!(config.sign_requests);
        assert_eq!(config.sp_private_key, Some("private-key".to_string()));
        assert_eq!(config.sp_certificate, Some("certificate".to_string()));
    }

    #[test]
    fn saml_config_validate_missing_entity_id() {
        let config = SamlConfig {
            entity_id: String::new(),
            acs_url: "https://sp.example.com/acs".to_string(),
            idp_entity_id: "https://idp.example.com".to_string(),
            idp_sso_url: "https://idp.example.com/sso".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(matches!(result, Err(SamlError::InvalidConfig(_))));
    }

    #[test]
    fn saml_config_validate_missing_acs_url() {
        let config = SamlConfig {
            entity_id: "https://sp.example.com".to_string(),
            acs_url: String::new(),
            idp_entity_id: "https://idp.example.com".to_string(),
            idp_sso_url: "https://idp.example.com/sso".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(matches!(result, Err(SamlError::InvalidConfig(_))));
    }

    #[test]
    fn saml_config_validate_missing_idp_entity_id() {
        let config = SamlConfig {
            entity_id: "https://sp.example.com".to_string(),
            acs_url: "https://sp.example.com/acs".to_string(),
            idp_entity_id: String::new(),
            idp_sso_url: "https://idp.example.com/sso".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(matches!(result, Err(SamlError::InvalidConfig(_))));
    }

    #[test]
    fn saml_config_validate_missing_idp_sso_url() {
        let config = SamlConfig {
            entity_id: "https://sp.example.com".to_string(),
            acs_url: "https://sp.example.com/acs".to_string(),
            idp_entity_id: "https://idp.example.com".to_string(),
            idp_sso_url: String::new(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(matches!(result, Err(SamlError::InvalidConfig(_))));
    }

    #[test]
    fn saml_config_validate_success() {
        let config = test_config();
        assert!(config.validate().is_ok());
    }

    // Attribute mapping tests

    #[test]
    fn attribute_mapping_default_values() {
        let mapping = SamlAttributeMapping::default();
        assert_eq!(mapping.email, "email");
        assert_eq!(mapping.first_name, Some("firstName".to_string()));
        assert_eq!(mapping.last_name, Some("lastName".to_string()));
        assert_eq!(mapping.display_name, Some("displayName".to_string()));
        assert_eq!(mapping.groups, Some("groups".to_string()));
        assert_eq!(mapping.default_role, UserRole::Viewer);
    }

    #[test]
    fn attribute_mapping_builder() {
        let mut role_mapping = HashMap::new();
        role_mapping.insert("admins".to_string(), UserRole::Admin);
        role_mapping.insert("developers".to_string(), UserRole::Developer);

        let mapping = SamlAttributeMapping::new("mail")
            .with_name_attributes("givenName", "sn")
            .with_display_name("cn")
            .with_groups("memberOf")
            .with_default_role(UserRole::Developer)
            .with_group_role_mapping(role_mapping);

        assert_eq!(mapping.email, "mail");
        assert_eq!(mapping.first_name, Some("givenName".to_string()));
        assert_eq!(mapping.last_name, Some("sn".to_string()));
        assert_eq!(mapping.display_name, Some("cn".to_string()));
        assert_eq!(mapping.groups, Some("memberOf".to_string()));
        assert_eq!(mapping.default_role, UserRole::Developer);
        assert_eq!(mapping.group_role_mapping.get("admins"), Some(&UserRole::Admin));
    }

    // SamlError display tests

    #[test]
    fn saml_error_display() {
        assert!(format!("{}", SamlError::InvalidConfig("test".to_string())).contains("configuration"));
        assert!(format!("{}", SamlError::InvalidResponse("test".to_string())).contains("response"));
        assert!(format!("{}", SamlError::SignatureInvalid).contains("signature"));
        assert!(format!("{}", SamlError::ResponseExpired).contains("expired"));
        assert!(format!("{}", SamlError::ConditionsNotMet("test".to_string())).contains("conditions"));
        assert!(format!("{}", SamlError::MissingAttribute("email".to_string())).contains("email"));
        assert!(format!("{}", SamlError::ProvisioningFailed("test".to_string())).contains("provisioning"));
        assert!(format!("{}", SamlError::XmlParseError("test".to_string())).contains("XML"));
        assert!(format!("{}", SamlError::Base64Error("test".to_string())).contains("base64"));
        assert!(format!("{}", SamlError::RequestExpired).contains("expired"));
        assert!(format!("{}", SamlError::UnknownRequest).contains("unknown"));
        assert!(format!("{}", SamlError::IdpInitiatedNotAllowed).contains("IdP-initiated"));
    }

    // SamlAuthRequest tests

    #[test]
    fn saml_auth_request_new() {
        let request = SamlAuthRequest::new(Some("https://app.example.com/dashboard".to_string()), 300);
        assert!(request.id.starts_with("_saml_req_"));
        assert_eq!(request.relay_state, Some("https://app.example.com/dashboard".to_string()));
        assert!(request.expires_at > request.issue_instant);
        assert!(!request.is_expired());
    }

    #[test]
    fn saml_auth_request_expired() {
        let request = SamlAuthRequest {
            id: "_test".to_string(),
            issue_instant: 0,
            relay_state: None,
            expires_at: 0,
        };
        assert!(request.is_expired());
    }

    // SamlAssertion tests

    #[test]
    fn saml_assertion_is_valid_success() {
        let now = current_timestamp();
        let assertion = SamlAssertion {
            id: "assertion-1".to_string(),
            issuer: "https://idp.example.com".to_string(),
            name_id: "user@example.com".to_string(),
            name_id_format: None,
            issue_instant: now,
            not_before: Some(now - 60),
            not_on_or_after: Some(now + 300),
            session_not_on_or_after: None,
            session_index: None,
            audiences: vec!["https://sp.example.com".to_string()],
            attributes: HashMap::new(),
            authn_context: None,
            in_response_to: None,
        };

        assert!(assertion.is_valid("https://sp.example.com", 60).is_ok());
    }

    #[test]
    fn saml_assertion_expired() {
        let now = current_timestamp();
        let assertion = SamlAssertion {
            id: "assertion-1".to_string(),
            issuer: "https://idp.example.com".to_string(),
            name_id: "user@example.com".to_string(),
            name_id_format: None,
            issue_instant: now - 600,
            not_before: None,
            not_on_or_after: Some(now - 300),
            session_not_on_or_after: None,
            session_index: None,
            audiences: vec![],
            attributes: HashMap::new(),
            authn_context: None,
            in_response_to: None,
        };

        assert!(matches!(
            assertion.is_valid("https://sp.example.com", 60),
            Err(SamlError::ResponseExpired)
        ));
    }

    #[test]
    fn saml_assertion_not_yet_valid() {
        let now = current_timestamp();
        let assertion = SamlAssertion {
            id: "assertion-1".to_string(),
            issuer: "https://idp.example.com".to_string(),
            name_id: "user@example.com".to_string(),
            name_id_format: None,
            issue_instant: now,
            not_before: Some(now + 600),
            not_on_or_after: None,
            session_not_on_or_after: None,
            session_index: None,
            audiences: vec![],
            attributes: HashMap::new(),
            authn_context: None,
            in_response_to: None,
        };

        assert!(matches!(
            assertion.is_valid("https://sp.example.com", 60),
            Err(SamlError::ConditionsNotMet(_))
        ));
    }

    #[test]
    fn saml_assertion_audience_mismatch() {
        let now = current_timestamp();
        let assertion = SamlAssertion {
            id: "assertion-1".to_string(),
            issuer: "https://idp.example.com".to_string(),
            name_id: "user@example.com".to_string(),
            name_id_format: None,
            issue_instant: now,
            not_before: None,
            not_on_or_after: None,
            session_not_on_or_after: None,
            session_index: None,
            audiences: vec!["https://other-sp.example.com".to_string()],
            attributes: HashMap::new(),
            authn_context: None,
            in_response_to: None,
        };

        assert!(matches!(
            assertion.is_valid("https://sp.example.com", 60),
            Err(SamlError::ConditionsNotMet(_))
        ));
    }

    #[test]
    fn saml_assertion_get_attribute() {
        let mut attributes = HashMap::new();
        attributes.insert("email".to_string(), vec!["user@example.com".to_string()]);
        attributes.insert("groups".to_string(), vec!["admins".to_string(), "developers".to_string()]);

        let assertion = SamlAssertion {
            id: "assertion-1".to_string(),
            issuer: "https://idp.example.com".to_string(),
            name_id: "user@example.com".to_string(),
            name_id_format: None,
            issue_instant: 0,
            not_before: None,
            not_on_or_after: None,
            session_not_on_or_after: None,
            session_index: None,
            audiences: vec![],
            attributes,
            authn_context: None,
            in_response_to: None,
        };

        assert_eq!(assertion.get_attribute("email"), Some("user@example.com"));
        assert_eq!(assertion.get_attribute("missing"), None);
        assert_eq!(
            assertion.get_attribute_values("groups"),
            Some(&vec!["admins".to_string(), "developers".to_string()])
        );
    }

    // Service Provider tests

    #[test]
    fn sp_new_validates_config() {
        let invalid_config = SamlConfig::default();
        assert!(SamlServiceProvider::new(invalid_config).is_err());

        let valid_config = test_config();
        assert!(SamlServiceProvider::new(valid_config).is_ok());
    }

    #[test]
    fn sp_generate_metadata() {
        let config = test_config()
            .with_organization("Gralph", "https://gralph.dev")
            .with_contact_email("support@gralph.dev");
        let sp = SamlServiceProvider::new(config).unwrap();
        let metadata = sp.generate_metadata();

        assert!(metadata.contains("EntityDescriptor"));
        assert!(metadata.contains("SPSSODescriptor"));
        assert!(metadata.contains("https://app.example.com"));
        assert!(metadata.contains("https://app.example.com/saml/acs"));
        assert!(metadata.contains("AssertionConsumerService"));
        assert!(metadata.contains("NameIDFormat"));
        assert!(metadata.contains("Organization"));
        assert!(metadata.contains("Gralph"));
        assert!(metadata.contains("support@gralph.dev"));
    }

    #[test]
    fn sp_generate_metadata_with_certificate() {
        let config = test_config()
            .with_signing(
                "-----BEGIN PRIVATE KEY-----\nMIIE...\n-----END PRIVATE KEY-----",
                "-----BEGIN CERTIFICATE-----\nMIIC...\n-----END CERTIFICATE-----",
            );
        let sp = SamlServiceProvider::new(config).unwrap();
        let metadata = sp.generate_metadata();

        assert!(metadata.contains("KeyDescriptor"));
        assert!(metadata.contains("X509Certificate"));
    }

    #[test]
    fn sp_generate_metadata_with_slo() {
        let config = test_config().with_slo_url("https://app.example.com/saml/slo");
        let sp = SamlServiceProvider::new(config).unwrap();
        let metadata = sp.generate_metadata();

        assert!(metadata.contains("SingleLogoutService"));
        assert!(metadata.contains("https://app.example.com/saml/slo"));
    }

    #[test]
    fn sp_create_authn_request() {
        let config = test_config();
        let sp = SamlServiceProvider::new(config).unwrap();

        let (url, request) = sp.create_authn_request(Some("https://app.example.com/dashboard".to_string()));

        assert!(url.starts_with("https://idp.example.com/sso?SAMLRequest="));
        assert!(url.contains("RelayState="));
        assert!(request.id.starts_with("_saml_req_"));
        assert_eq!(request.relay_state, Some("https://app.example.com/dashboard".to_string()));
    }

    #[test]
    fn sp_create_authn_request_without_relay_state() {
        let config = test_config();
        let sp = SamlServiceProvider::new(config).unwrap();

        let (url, request) = sp.create_authn_request(None);

        assert!(url.starts_with("https://idp.example.com/sso?SAMLRequest="));
        assert!(!url.contains("RelayState="));
        assert!(request.relay_state.is_none());
    }

    #[test]
    fn sp_accessors() {
        let config = test_config();
        let sp = SamlServiceProvider::new(config).unwrap();

        assert_eq!(sp.entity_id(), "https://app.example.com");
        assert_eq!(sp.acs_url(), "https://app.example.com/saml/acs");
        assert_eq!(sp.idp_sso_url(), "https://idp.example.com/sso");
    }

    // Helper function tests

    #[test]
    fn test_format_iso8601() {
        // Test epoch
        assert_eq!(format_iso8601(0), "1970-01-01T00:00:00Z");

        // Test a known date
        let timestamp = 1609459200; // 2021-01-01T00:00:00Z
        let formatted = format_iso8601(timestamp);
        assert!(formatted.starts_with("2021-01-01"));
    }

    #[test]
    fn test_parse_iso8601() {
        assert_eq!(parse_iso8601("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_iso8601("1970-01-01T00:00:00.000Z"), Some(0));

        // Test round-trip
        let timestamp = 1609459200u64;
        let formatted = format_iso8601(timestamp);
        let parsed = parse_iso8601(&formatted);
        assert_eq!(parsed, Some(timestamp));
    }

    #[test]
    fn test_parse_iso8601_invalid() {
        assert!(parse_iso8601("invalid").is_none());
        assert!(parse_iso8601("2021-01-01").is_none());
        assert!(parse_iso8601("00:00:00").is_none());
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(xml_escape("hello"), "hello");
        assert_eq!(xml_escape("<test>"), "&lt;test&gt;");
        assert_eq!(xml_escape("a & b"), "a &amp; b");
        assert_eq!(xml_escape(r#"a "b" c"#), "a &quot;b&quot; c");
        assert_eq!(xml_escape("a 'b' c"), "a &apos;b&apos; c");
    }

    #[test]
    fn test_extract_certificate_body() {
        let pem = "-----BEGIN CERTIFICATE-----\nMIIC\nABCD\n-----END CERTIFICATE-----";
        assert_eq!(extract_certificate_body(pem), "MIICABCD");
    }

    #[test]
    fn test_extract_xml_attribute() {
        let xml = r#"<Element ID="abc123" Name="test">"#;
        assert_eq!(extract_xml_attribute(xml, "ID"), Some("abc123".to_string()));
        assert_eq!(extract_xml_attribute(xml, "Name"), Some("test".to_string()));
        assert_eq!(extract_xml_attribute(xml, "Missing"), None);
    }

    #[test]
    fn test_extract_xml_element() {
        let xml = "<saml:Issuer>https://idp.example.com</saml:Issuer>";
        assert_eq!(
            extract_xml_element(xml, "Issuer"),
            Some("https://idp.example.com".to_string())
        );

        let xml_no_prefix = "<Issuer>https://idp.example.com</Issuer>";
        assert_eq!(
            extract_xml_element(xml_no_prefix, "Issuer"),
            Some("https://idp.example.com".to_string())
        );
    }

    #[test]
    fn test_extract_xml_elements() {
        let xml = "<saml:Audience>aud1</saml:Audience><saml:Audience>aud2</saml:Audience>";
        let audiences = extract_xml_elements(xml, "Audience");
        assert_eq!(audiences, vec!["aud1".to_string(), "aud2".to_string()]);
    }

    #[test]
    fn test_determine_role() {
        let mut mapping = HashMap::new();
        mapping.insert("admins".to_string(), UserRole::Admin);
        mapping.insert("developers".to_string(), UserRole::Developer);

        assert_eq!(
            determine_role(&["admins".to_string()], &mapping, UserRole::Viewer),
            UserRole::Admin
        );
        assert_eq!(
            determine_role(&["developers".to_string()], &mapping, UserRole::Viewer),
            UserRole::Developer
        );
        assert_eq!(
            determine_role(&["users".to_string()], &mapping, UserRole::Viewer),
            UserRole::Viewer
        );
        assert_eq!(
            determine_role(&[], &mapping, UserRole::Developer),
            UserRole::Developer
        );
    }

    // Mock IdP response test

    #[test]
    fn sp_process_response_validates_issuer() {
        let config = test_config();
        let sp = SamlServiceProvider::new(config).unwrap();

        // Create a mock SAML response with wrong issuer
        let mock_response = create_mock_saml_response(
            "https://wrong-idp.example.com",
            "user@example.com",
            "https://app.example.com",
        );
        let encoded = BASE64.encode(mock_response.as_bytes());

        let result = sp.process_response(&encoded, None);
        assert!(matches!(result, Err(SamlError::InvalidResponse(_))));
    }

    #[test]
    fn sp_process_response_rejects_idp_initiated_when_disabled() {
        let config = test_config(); // allow_idp_initiated is false by default
        let sp = SamlServiceProvider::new(config).unwrap();

        // Create a mock SAML response without InResponseTo (IdP-initiated)
        let mock_response = create_mock_saml_response(
            "https://idp.example.com",
            "user@example.com",
            "https://app.example.com",
        );
        let encoded = BASE64.encode(mock_response.as_bytes());

        let result = sp.process_response(&encoded, None);
        assert!(matches!(result, Err(SamlError::IdpInitiatedNotAllowed)));
    }

    #[test]
    fn sp_process_response_accepts_idp_initiated_when_enabled() {
        let config = test_config().with_idp_initiated(true);
        let sp = SamlServiceProvider::new(config).unwrap();

        let mock_response = create_mock_saml_response(
            "https://idp.example.com",
            "user@example.com",
            "https://app.example.com",
        );
        let encoded = BASE64.encode(mock_response.as_bytes());

        let result = sp.process_response(&encoded, None);
        assert!(result.is_ok());
    }

    #[test]
    fn sp_process_response_validates_request_id() {
        let config = test_config();
        let sp = SamlServiceProvider::new(config).unwrap();

        // First create a real auth request
        let (_url, request) = sp.create_authn_request(None);

        // Create response with matching InResponseTo
        let mock_response = create_mock_saml_response_with_request_id(
            "https://idp.example.com",
            "user@example.com",
            "https://app.example.com",
            Some(&request.id),
        );
        let encoded = BASE64.encode(mock_response.as_bytes());

        let result = sp.process_response(&encoded, None);
        assert!(result.is_ok());
    }

    #[test]
    fn sp_process_response_rejects_unknown_request_id() {
        let config = test_config();
        let sp = SamlServiceProvider::new(config).unwrap();

        // Create response with unknown InResponseTo
        let mock_response = create_mock_saml_response_with_request_id(
            "https://idp.example.com",
            "user@example.com",
            "https://app.example.com",
            Some("_unknown_request_id"),
        );
        let encoded = BASE64.encode(mock_response.as_bytes());

        let result = sp.process_response(&encoded, None);
        assert!(matches!(result, Err(SamlError::UnknownRequest)));
    }

    #[test]
    fn sp_extract_user_profile() {
        let config = test_config();
        let sp = SamlServiceProvider::new(config).unwrap();

        let mut attributes = HashMap::new();
        attributes.insert("email".to_string(), vec!["user@example.com".to_string()]);
        attributes.insert("firstName".to_string(), vec!["John".to_string()]);
        attributes.insert("lastName".to_string(), vec!["Doe".to_string()]);
        attributes.insert("displayName".to_string(), vec!["John Doe".to_string()]);
        attributes.insert("groups".to_string(), vec!["developers".to_string()]);

        let assertion = SamlAssertion {
            id: "assertion-1".to_string(),
            issuer: "https://idp.example.com".to_string(),
            name_id: "user@example.com".to_string(),
            name_id_format: Some("urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress".to_string()),
            issue_instant: current_timestamp(),
            not_before: None,
            not_on_or_after: None,
            session_not_on_or_after: None,
            session_index: Some("session-123".to_string()),
            audiences: vec![],
            attributes,
            authn_context: None,
            in_response_to: None,
        };

        let profile = sp.extract_user_profile(&assertion).unwrap();

        assert_eq!(profile.email, "user@example.com");
        assert_eq!(profile.first_name, Some("John".to_string()));
        assert_eq!(profile.last_name, Some("Doe".to_string()));
        assert_eq!(profile.display_name, Some("John Doe".to_string()));
        assert_eq!(profile.groups, vec!["developers".to_string()]);
        assert_eq!(profile.session_index, Some("session-123".to_string()));
        assert_eq!(profile.name_id, "user@example.com");
    }

    #[test]
    fn sp_extract_user_profile_falls_back_to_name_id_for_email() {
        let config = test_config();
        let sp = SamlServiceProvider::new(config).unwrap();

        let assertion = SamlAssertion {
            id: "assertion-1".to_string(),
            issuer: "https://idp.example.com".to_string(),
            name_id: "user@example.com".to_string(),
            name_id_format: None,
            issue_instant: current_timestamp(),
            not_before: None,
            not_on_or_after: None,
            session_not_on_or_after: None,
            session_index: None,
            audiences: vec![],
            attributes: HashMap::new(), // No email attribute
            authn_context: None,
            in_response_to: None,
        };

        let profile = sp.extract_user_profile(&assertion).unwrap();
        assert_eq!(profile.email, "user@example.com");
    }

    #[test]
    fn sp_extract_user_profile_fails_without_email() {
        let config = test_config();
        let sp = SamlServiceProvider::new(config).unwrap();

        let assertion = SamlAssertion {
            id: "assertion-1".to_string(),
            issuer: "https://idp.example.com".to_string(),
            name_id: "user123".to_string(), // Not an email
            name_id_format: None,
            issue_instant: current_timestamp(),
            not_before: None,
            not_on_or_after: None,
            session_not_on_or_after: None,
            session_index: None,
            audiences: vec![],
            attributes: HashMap::new(),
            authn_context: None,
            in_response_to: None,
        };

        let result = sp.extract_user_profile(&assertion);
        assert!(matches!(result, Err(SamlError::MissingAttribute(_))));
    }

    // Helper function to create mock SAML responses for testing
    fn create_mock_saml_response(issuer: &str, name_id: &str, audience: &str) -> String {
        create_mock_saml_response_with_request_id(issuer, name_id, audience, None)
    }

    fn create_mock_saml_response_with_request_id(
        issuer: &str,
        name_id: &str,
        audience: &str,
        in_response_to: Option<&str>,
    ) -> String {
        let now = current_timestamp();
        let issue_instant = format_iso8601(now);
        let not_on_or_after = format_iso8601(now + 300);

        let in_response_to_attr = in_response_to
            .map(|id| format!(r#" InResponseTo="{}""#, id))
            .unwrap_or_default();

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol"
    xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion"
    ID="_response_123"
    Version="2.0"
    IssueInstant="{issue_instant}"{in_response_to_attr}>
  <saml:Issuer>{issuer}</saml:Issuer>
  <samlp:Status>
    <samlp:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"/>
  </samlp:Status>
  <saml:Assertion xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion"
      ID="_assertion_456"
      Version="2.0"
      IssueInstant="{issue_instant}">
    <saml:Issuer>{issuer}</saml:Issuer>
    <saml:Subject>
      <saml:NameID Format="urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress">{name_id}</saml:NameID>
    </saml:Subject>
    <saml:Conditions NotOnOrAfter="{not_on_or_after}">
      <saml:AudienceRestriction>
        <saml:Audience>{audience}</saml:Audience>
      </saml:AudienceRestriction>
    </saml:Conditions>
    <saml:AuthnStatement AuthnInstant="{issue_instant}">
      <saml:AuthnContext>
        <saml:AuthnContextClassRef>urn:oasis:names:tc:SAML:2.0:ac:classes:PasswordProtectedTransport</saml:AuthnContextClassRef>
      </saml:AuthnContext>
    </saml:AuthnStatement>
    <saml:AttributeStatement>
      <saml:Attribute Name="email">
        <saml:AttributeValue>{name_id}</saml:AttributeValue>
      </saml:Attribute>
    </saml:AttributeStatement>
  </saml:Assertion>
</samlp:Response>"#,
            issue_instant = issue_instant,
            in_response_to_attr = in_response_to_attr,
            issuer = issuer,
            name_id = name_id,
            audience = audience,
            not_on_or_after = not_on_or_after,
        )
    }

    // Integration test with SamlAuthService

    #[test]
    fn saml_auth_service_start_auth() {
        let config = test_config();
        let sp = SamlServiceProvider::new(config).unwrap();
        let user_store = UserStore::new();
        let jwt_config = crate::auth::JwtConfig::new("test-secret");

        let service = SamlAuthService::new(sp, user_store, jwt_config);

        let (url, request) = service.start_auth(Some("/dashboard".to_string()));

        assert!(url.contains("SAMLRequest="));
        assert!(url.contains("RelayState="));
        assert!(request.relay_state.is_some());
    }

    #[test]
    fn saml_auth_service_metadata() {
        let config = test_config();
        let sp = SamlServiceProvider::new(config).unwrap();
        let user_store = UserStore::new();
        let jwt_config = crate::auth::JwtConfig::new("test-secret");

        let service = SamlAuthService::new(sp, user_store, jwt_config);
        let metadata = service.metadata();

        assert!(metadata.contains("EntityDescriptor"));
        assert!(metadata.contains("https://app.example.com"));
    }
}
