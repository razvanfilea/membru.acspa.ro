import {useEffect} from "react";
import {MemberTypes} from "../types/wrapper";
import {useRouter} from "next/router";
import useProfileData from "./useProfileData";

export default function useExitIfNotFounder() {
    const profileData = useProfileData()
    const router = useRouter()

    useEffect(() => {
        if (!profileData.isLoading &&
            (profileData.profile == null || profileData.profile.role !== MemberTypes.Fondator))
            router.back()
    }, [profileData, router])
}
