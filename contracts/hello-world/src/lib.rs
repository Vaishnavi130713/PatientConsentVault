#![allow(non_snake_case)]
#![no_std]
use soroban_sdk::{contract, contracttype, contractimpl, log, Env, Symbol, String, symbol_short, Address};


// Structure to track overall consent vault statistics
#[contracttype]
#[derive(Clone)]
pub struct VaultStats {
    pub total_patients: u64,
    pub total_consents: u64,
    pub active_consents: u64,
    pub revoked_consents: u64,
}


const VAULT_STATS: Symbol = symbol_short!("V_STATS");


// Consent Status
#[contracttype]
#[derive(Clone, PartialEq)]
pub enum ConsentStatus {
    Active,
    Revoked,
    Expired,
}


// Access Purpose
#[contracttype]
#[derive(Clone, PartialEq)]
pub enum AccessPurpose {
    Treatment,
    Research,
    Billing,
    Emergency,
    Other,
}


// Mapping consent_id to Consent struct
#[contracttype] 
pub enum ConsentBook { 
    Consent(u64)
}


const COUNT_CONSENT: Symbol = symbol_short!("C_CONSEN"); 


// Structure defining a patient consent
#[contracttype]
#[derive(Clone)] 
pub struct Consent {
    pub consent_id: u64,
    pub patient: Address,
    pub provider: Address,
    pub purpose: AccessPurpose,
    pub data_category: String,         // e.g., "Medical History", "Lab Results", "Prescriptions"
    pub grant_date: u64,
    pub expiry_date: u64,
    pub status: ConsentStatus,
    pub scope_description: String,
}


// Mapping patient address to PatientProfile
#[contracttype] 
pub enum PatientBook { 
    Patient(Address)
}


// Structure defining a patient profile
#[contracttype]
#[derive(Clone)] 
pub struct PatientProfile {
    pub patient: Address,
    pub patient_name: String,
    pub registration_date: u64,
    pub total_consents_granted: u64,
    pub active_consents_count: u64,
}


// Mapping to track access logs
#[contracttype] 
pub enum AccessLog { 
    Log(u64)
}


const COUNT_ACCESS: Symbol = symbol_short!("C_ACCESS"); 


// Structure for tracking data access
#[contracttype]
#[derive(Clone)] 
pub struct DataAccess {
    pub access_id: u64,
    pub consent_id: u64,
    pub provider: Address,
    pub access_date: u64,
    pub access_purpose: AccessPurpose,
}


#[contract]
pub struct PatientConsentVault;

#[contractimpl]
impl PatientConsentVault {

    // Function 1: Register patient in consent vault
    pub fn register_patient(
        env: Env,
        patient: Address,
        patient_name: String
    ) {
        patient.require_auth();
        
        let existing_patient = Self::view_patient_profile(env.clone(), patient.clone());
        
        if existing_patient.registration_date > 0 {
            log!(&env, "Patient already registered!");
            panic!("Patient already exists!");
        }
        
        let current_time = env.ledger().timestamp();
        
        let new_patient = PatientProfile {
            patient: patient.clone(),
            patient_name: patient_name.clone(),
            registration_date: current_time,
            total_consents_granted: 0,
            active_consents_count: 0,
        };
        
        let mut stats = Self::view_vault_stats(env.clone());
        stats.total_patients += 1;
        
        env.storage().instance().set(&PatientBook::Patient(patient.clone()), &new_patient);
        env.storage().instance().set(&VAULT_STATS, &stats);
        env.storage().instance().extend_ttl(17280, 17280);
        
        log!(&env, "Patient Registered - Name: {}", patient_name);
    }


    // Function 2: Grant consent to healthcare provider
    pub fn grant_consent(
        env: Env,
        patient: Address,
        provider: Address,
        purpose: AccessPurpose,
        data_category: String,
        validity_days: u64,
        scope_description: String
    ) -> u64 {
        patient.require_auth();
        
        let mut patient_profile = Self::view_patient_profile(env.clone(), patient.clone());
        
        if patient_profile.registration_date == 0 {
            log!(&env, "Patient not registered!");
            panic!("Patient not registered!");
        }
        
        let mut count_consent: u64 = env.storage().instance().get(&COUNT_CONSENT).unwrap_or(0);
        count_consent += 1;
        
        let current_time = env.ledger().timestamp();
        let expiry_date = current_time + (validity_days * 86400);
        
        let new_consent = Consent {
            consent_id: count_consent,
            patient: patient.clone(),
            provider: provider.clone(),
            purpose: purpose.clone(),
            data_category: data_category.clone(),
            grant_date: current_time,
            expiry_date,
            status: ConsentStatus::Active,
            scope_description,
        };
        
        patient_profile.total_consents_granted += 1;
        patient_profile.active_consents_count += 1;
        
        let mut stats = Self::view_vault_stats(env.clone());
        stats.total_consents += 1;
        stats.active_consents += 1;
        
        env.storage().instance().set(&ConsentBook::Consent(count_consent), &new_consent);
        env.storage().instance().set(&COUNT_CONSENT, &count_consent);
        env.storage().instance().set(&PatientBook::Patient(patient), &patient_profile);
        env.storage().instance().set(&VAULT_STATS, &stats);
        env.storage().instance().extend_ttl(17280, 17280);
        
        log!(&env, "Consent Granted - ID: {}, Provider: {}, Purpose: {:?}", count_consent, provider, purpose);
        
        return count_consent;
    }


    // Function 3: Revoke consent
    pub fn revoke_consent(
        env: Env,
        consent_id: u64,
        patient: Address
    ) {
        patient.require_auth();
        
        let mut consent = Self::view_consent(env.clone(), consent_id);
        
        if consent.consent_id == 0 {
            log!(&env, "Consent not found!");
            panic!("Consent not found!");
        }
        
        if consent.patient != patient {
            log!(&env, "Unauthorized! Not the consent owner.");
            panic!("Unauthorized!");
        }
        
        if consent.status != ConsentStatus::Active {
            log!(&env, "Consent is not active!");
            panic!("Consent not active!");
        }
        
        consent.status = ConsentStatus::Revoked;
        
        let mut patient_profile = Self::view_patient_profile(env.clone(), patient.clone());
        if patient_profile.active_consents_count > 0 {
            patient_profile.active_consents_count -= 1;
        }
        
        let mut stats = Self::view_vault_stats(env.clone());
        if stats.active_consents > 0 {
            stats.active_consents -= 1;
        }
        stats.revoked_consents += 1;
        
        env.storage().instance().set(&ConsentBook::Consent(consent_id), &consent);
        env.storage().instance().set(&PatientBook::Patient(patient), &patient_profile);
        env.storage().instance().set(&VAULT_STATS, &stats);
        env.storage().instance().extend_ttl(17280, 17280);
        
        log!(&env, "Consent Revoked - ID: {}", consent_id);
    }


    // Function 4: Log data access by provider
    pub fn log_data_access(
        env: Env,
        consent_id: u64,
        provider: Address,
        access_purpose: AccessPurpose
    ) -> u64 {
        provider.require_auth();
        
        let consent = Self::view_consent(env.clone(), consent_id);
        
        if consent.consent_id == 0 {
            log!(&env, "Consent not found!");
            panic!("Consent not found!");
        }
        
        if consent.provider != provider {
            log!(&env, "Unauthorized provider!");
            panic!("Unauthorized provider!");
        }
        
        if consent.status != ConsentStatus::Active {
            log!(&env, "Consent is not active!");
            panic!("Consent not active!");
        }
        
        let current_time = env.ledger().timestamp();
        
        if current_time > consent.expiry_date {
            log!(&env, "Consent has expired!");
            panic!("Consent expired!");
        }
        
        if consent.purpose != access_purpose && consent.purpose != AccessPurpose::Other {
            log!(&env, "Access purpose does not match consent!");
            panic!("Purpose mismatch!");
        }
        
        let mut count_access: u64 = env.storage().instance().get(&COUNT_ACCESS).unwrap_or(0);
        count_access += 1;
        
        let access_record = DataAccess {
            access_id: count_access,
            consent_id,
            provider: provider.clone(),
            access_date: current_time,
            access_purpose: access_purpose.clone(),
        };
        
        env.storage().persistent().set(&AccessLog::Log(count_access), &access_record);
        env.storage().instance().set(&COUNT_ACCESS, &count_access);
        env.storage().instance().extend_ttl(17280, 17280);
        
        log!(&env, "Data Access Logged - Consent: {}, Provider: {}, Purpose: {:?}", consent_id, provider, access_purpose);
        
        return count_access;
    }


    // View function: Get consent details
    pub fn view_consent(env: Env, consent_id: u64) -> Consent {
        let key = ConsentBook::Consent(consent_id);
        
        env.storage().instance().get(&key).unwrap_or(Consent {
            consent_id: 0,
            patient: Address::from_string(&String::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF")),
            provider: Address::from_string(&String::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF")),
            purpose: AccessPurpose::Other,
            data_category: String::from_str(&env, "Not_Found"),
            grant_date: 0,
            expiry_date: 0,
            status: ConsentStatus::Expired,
            scope_description: String::from_str(&env, "Not_Found"),
        })
    }


    // View function: Get patient profile
    pub fn view_patient_profile(env: Env, patient: Address) -> PatientProfile {
        let key = PatientBook::Patient(patient.clone());
        
        env.storage().instance().get(&key).unwrap_or(PatientProfile {
            patient,
            patient_name: String::from_str(&env, "Not_Found"),
            registration_date: 0,
            total_consents_granted: 0,
            active_consents_count: 0,
        })
    }


    // View function: Get access log
    pub fn view_access_log(env: Env, access_id: u64) -> DataAccess {
        let key = AccessLog::Log(access_id);
        
        env.storage().persistent().get(&key).unwrap_or(DataAccess {
            access_id: 0,
            consent_id: 0,
            provider: Address::from_string(&String::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF")),
            access_date: 0,
            access_purpose: AccessPurpose::Other,
        })
    }


    // View function: Get vault statistics
    pub fn view_vault_stats(env: Env) -> VaultStats {
        env.storage().instance().get(&VAULT_STATS).unwrap_or(VaultStats {
            total_patients: 0,
            total_consents: 0,
            active_consents: 0,
            revoked_consents: 0,
        })
    }


    // Helper: Check if consent is valid
    pub fn verify_consent_validity(env: Env, consent_id: u64) -> bool {
        let consent = Self::view_consent(env.clone(), consent_id);
        
        if consent.consent_id == 0 {
            return false;
        }
        
        let current_time = env.ledger().timestamp();
        
        return consent.status == ConsentStatus::Active && current_time <= consent.expiry_date;
    }
}
