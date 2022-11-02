import React, {useCallback, useContext, useEffect, useState} from "react";
import {Profile} from "../../types/wrapper";
import {User, useSession, useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../../types/database.types";

export interface ProfileData {
    isLoading: boolean,
    profile: Profile | null
}

const defaultValue: ProfileData = {
    isLoading: true,
    profile: null,
}
const AuthContext = React.createContext(defaultValue)

export default function ProfileProvider({children}) {
    const supabase = useSupabaseClient<Database>()
    const session = useSession()
    const [profile, setProfile] = useState<Profile | null>(null)
    const [loading, setLoading] = useState(true)

    useEffect(() => {
        async function getProfile(user: User) {
            try {
                setLoading(true)

                let { data, error} = await supabase
                    .from('profiles')
                    .select('*')
                    .eq('id', user.id)
                    .single()

                if (error) {
                    console.log(error)
                } else {
                    setProfile(data);
                }

            } catch (error) {
                alert('Error loading user data!')
                console.log(error)
            } finally {
                setLoading(false)
            }
        }

        if (session?.user) {
            getProfile(session!.user).then(() => console.log("Profile loaded"))
        } else {
            setLoading(false)
        }

        const {data: listener} = supabase.auth.onAuthStateChange(
            async (event, session) => {
                if (event == "PASSWORD_RECOVERY") {
                    let newPassword: string | null = null;
                    while (!newPassword || newPassword.length < 8)
                        newPassword = prompt("Noua ta parolă")

                    const {data, error} = await supabase.auth.updateUser({
                        password: newPassword,
                    })

                    if (data) alert("Parola ta a fost schimbată")
                    if (error) alert("A apărut o eroare la actualizarea parolei")
                }
            }
        )

        return () => {
            listener?.subscription?.unsubscribe()
        }
    }, [session, supabase]);

    // Will be passed down to Signup, Login and Dashboard components
    const authDataCallback = useCallback((): ProfileData => {
        return {
            isLoading: loading,
            profile
        }
    }, [loading, profile])

    return (
        <AuthContext.Provider value={authDataCallback()}>
            {children}
        </AuthContext.Provider>
    )
}

export function useProfile() {
    return useContext(AuthContext)
}
