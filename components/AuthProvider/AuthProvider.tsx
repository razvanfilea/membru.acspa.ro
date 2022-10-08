import React, {useCallback, useContext, useEffect, useState} from "react";
import {supabase} from "../../utils/supabase_utils";
import {ApiError, Provider, Session, User} from "@supabase/gotrue-js";
import {Profile} from "../../types/wrapper";

export interface LoginCredentials {
    email: string
    password: string
}

export interface AuthData {
    changePassword: (userName: string | null, password: string) => Promise<boolean>
    signIn: (credentials: LoginCredentials) => Promise<{
        session: Session | null
        user: User | null
        provider?: Provider
        url?: string | null
        error: ApiError | null
    }>,
    signOut: () => Promise<{ error: ApiError | null }>
    isLoading: boolean,
    user: User | null
    profile: Profile | null
}

async function changePasswordAsync(userName: string | null, password: string): Promise<boolean> {
    console.log("Updating password")
    const {error, user} = await supabase.auth.update({password})

    if (user != null) {
        if (userName != null) {
            console.log("Updating profile")

            const profile: Profile = {
                id: user.id,
                name: userName
            }

            await supabase.from<Profile>('profiles').insert([profile])
        }
        return true
    }
    console.log("Failed to change password", error)

    return false
}

const defaultValue: AuthData = {
    changePassword: changePasswordAsync,
    signIn: (credentials) => supabase.auth.signIn(credentials),
    signOut: () => supabase.auth.signOut(),
    isLoading: true,
    user: null,
    profile: null,
}
const AuthContext = React.createContext(defaultValue)

export default function AuthProvider({children}) {
    const [user, setUser] = useState<User | null>(null)
    const [profile, setProfile] = useState<Profile | null>(null)
    const [loading, setLoading] = useState(true)

    useEffect(() => {
        async function updateProfile(newUser) {
            if (newUser == null) {
                setProfile(null)
                return;
            }

            const {error, data: newProfile} = await supabase
                .from<Profile>("profiles")
                .select("*")
                .eq("id", newUser.id)
                .limit(1)
                .maybeSingle()

            if (error == null) {
                setProfile(newProfile)
            } else {
                console.log(error)
            }
        }

        // Check active sessions and sets the user
        const session = supabase.auth.session()
        setUser(session?.user ?? null)
        updateProfile(session?.user ?? null).then(() => {
            setLoading(false)
        })

        // Listen for changes on auth state (logged in, signed out, etc.)
        const {data: listener} = supabase.auth.onAuthStateChange(
            async (event, session) => {
                if (event == "PASSWORD_RECOVERY") {
                    let newPassword: string | null = null;
                    while (!newPassword || newPassword.length < 8)
                        newPassword = prompt("Noua ta parolă")

                    const {data, error} = await supabase.auth.update({
                        password: newPassword,
                    })

                    if (data) alert("Parola ta a fost schimbată")
                    if (error) alert("A apărut o eroare la actualizarea parolei")
                } else {
                    const newUser = session?.user ?? null;
                    setUser(newUser)
                    await updateProfile(newUser)
                    setLoading(false)
                }
            }
        )

        return () => {
            listener?.unsubscribe()
        }
    }, [])

    // Will be passed down to Signup, Login and Dashboard components
    const authDataCallback = useCallback((): AuthData => {
        return {
            changePassword: changePasswordAsync,
            signIn: (credentials) => supabase.auth.signIn(credentials),
            signOut: () => supabase.auth.signOut(),
            isLoading: loading,
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
