export type Json =
  | string
  | number
  | boolean
  | null
  | { [key: string]: Json | undefined }
  | Json[]

export interface Database {
  public: {
    Tables: {
      admin_vars: {
        Row: {
          service_role: string
        }
        Insert: {
          service_role?: string
        }
        Update: {
          service_role?: string
        }
        Relationships: []
      }
      guests: {
        Row: {
          created_at: string
          guest_name: string
          special: boolean
          start_date: string
          start_hour: number
          user_id: string
        }
        Insert: {
          created_at?: string
          guest_name: string
          special: boolean
          start_date: string
          start_hour: number
          user_id?: string
        }
        Update: {
          created_at?: string
          guest_name?: string
          special?: boolean
          start_date?: string
          start_hour?: number
          user_id?: string
        }
        Relationships: [
          {
            foreignKeyName: "guests_user_id_fkey"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          }
        ]
      }
      locations: {
        Row: {
          end_hour: number
          max_reservations: number
          name: string
          reservation_duration: number
          start_hour: number
          weekend_end_hour: number
          weekend_reservation_duration: number
          weekend_start_hour: number
        }
        Insert: {
          end_hour: number
          max_reservations: number
          name: string
          reservation_duration: number
          start_hour: number
          weekend_end_hour?: number
          weekend_reservation_duration?: number
          weekend_start_hour?: number
        }
        Update: {
          end_hour?: number
          max_reservations?: number
          name?: string
          reservation_duration?: number
          start_hour?: number
          weekend_end_hour?: number
          weekend_reservation_duration?: number
          weekend_start_hour?: number
        }
        Relationships: []
      }
      member_roles: {
        Row: {
          role: string
        }
        Insert: {
          role: string
        }
        Update: {
          role?: string
        }
        Relationships: []
      }
      profiles: {
        Row: {
          has_key: boolean
          id: string
          name: string
          role: string
        }
        Insert: {
          has_key?: boolean
          id: string
          name: string
          role?: string
        }
        Update: {
          has_key?: boolean
          id?: string
          name?: string
          role?: string
        }
        Relationships: [
          {
            foreignKeyName: "profiles_id_fkey"
            columns: ["id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          },
          {
            foreignKeyName: "profiles_role_fkey"
            columns: ["role"]
            referencedRelation: "member_roles"
            referencedColumns: ["role"]
          }
        ]
      }
      reservations_restrictions: {
        Row: {
          date: string
          message: string
          start_hour: number
          user_id: string
        }
        Insert: {
          date: string
          message: string
          start_hour: number
          user_id?: string
        }
        Update: {
          date?: string
          message?: string
          start_hour?: number
          user_id?: string
        }
        Relationships: [
          {
            foreignKeyName: "reservations_restrictions_user_id_fkey"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          }
        ]
      }
      rezervari: {
        Row: {
          cancelled: boolean
          created_at: string
          duration: number
          id: string
          location: string
          start_date: string
          start_hour: number
          user_id: string
        }
        Insert: {
          cancelled?: boolean
          created_at?: string
          duration: number
          id?: string
          location: string
          start_date: string
          start_hour: number
          user_id: string
        }
        Update: {
          cancelled?: boolean
          created_at?: string
          duration?: number
          id?: string
          location?: string
          start_date?: string
          start_hour?: number
          user_id?: string
        }
        Relationships: [
          {
            foreignKeyName: "rezervari_location_fkey"
            columns: ["location"]
            referencedRelation: "locations"
            referencedColumns: ["name"]
          },
          {
            foreignKeyName: "rezervari_user_id_fkey"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          }
        ]
      }
    }
    Views: {
      [_ in never]: never
    }
    Functions: {
      create_guest_from_current_user: {
        Args: {
          start_date_input: string
          start_hour_input: number
        }
        Returns: undefined
      }
      create_reservation: {
        Args: {
          location_input: string
          start_date_input: string
          start_hour_input: number
        }
        Returns: string
      }
    }
    Enums: {
      [_ in never]: never
    }
    CompositeTypes: {
      [_ in never]: never
    }
  }
}
