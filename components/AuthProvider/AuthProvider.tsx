import React, {useCallback, useContext, useEffect, useState} from "react";
import {supabase} from "../../utils/supabase_utils";
import {User, ApiError, Provider, Session} from "@supabase/gotrue-js";
import {definitions} from "../../types/supabase";
import {Profile} from "../../types/wrapper";

export interface LoginCredentials {
    email: string
    password: string
}

export interface AuthData {
    signUp: (credentials: LoginCredentials, name: string) => Promise<boolean>,
    signIn: (credentials: LoginCredentials) => Promise<{
        session: Session | null
        user: User | null
        provider?: Provider
        url?: string | null
        error: ApiError | null
    }>,
    signOut: () => Promise<{ error: ApiError | null }>
    loading: boolean,
    user: User,
    profile: definitions["profiles"]
}

async function signUpAsync(credentials: LoginCredentials, name: string): Promise<boolean> {
    const {user, session, error} = await supabase.auth.signUp(credentials)
    if (user != null || session != null) {
        await supabase.auth.signIn(credentials)
        await supabase.from('profiles')
            .insert([
                {name}
            ])
        return true
    }

    return false
}

const default_value: AuthData = {
    signUp: (credentials, name) => signUpAsync(credentials, name),
    signIn: (credentials) => supabase.auth.signIn(credentials),
    signOut: () => supabase.auth.signOut(),
    loading: true,
    user: null,
    profile: null,
}
const AuthContext = React.createContext(default_value)

export default function AuthProvider({children}) {
    const [user, setUser] = useState<User>(null)
    const [profile, setProfile] = useState<Profile>(null)
    const [loading, setLoading] = useState(true)

    useEffect(() => {
        async function updateProfile(newUser) {
            if (newUser == null)
                return;

            const newProfile = await supabase
                .from<Profile>("profiles")
                .select("*")
                .eq("id", newUser.id)[0]
            setProfile(newProfile)
        }

        // Check active sessions and sets the user
        const session = supabase.auth.session()
        setUser(session?.user ?? null)
        updateProfile(session?.user ?? null)
        setLoading(false)


        // Listen for changes on auth state (logged in, signed out, etc.)
        const {data: listener} = supabase.auth.onAuthStateChange(
            async (event, session) => {
                const newUser = session?.user ?? null;
                setUser(newUser)
                updateProfile(session?.user ?? null)
                setLoading(false)
            }
        )

        return () => {
            listener?.unsubscribe()
        }
    }, [])

    // Will be passed down to Signup, Login and Dashboard components
    const authDataCallback = useCallback((): AuthData => {
        return {
            signUp: signUpAsync,
            signIn: (credentials) => supabase.auth.signIn(credentials),
            signOut: () => supabase.auth.signOut(),
            loading,
            user,
            profile
        }
    }, [loading, profile, user])

    return (
        <AuthContext.Provider value={authDataCallback()}>
            {children}
        </AuthContext.Provider>
    )
}

export function useAuth() {
    return useContext(AuthContext)
}
